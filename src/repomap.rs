#![allow(
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::manual_let_else
)]

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser, Tree};

use crate::defaults;
use crate::safety;

/// A code symbol extracted from a source file.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Class,
    Method,
    Import,
}

/// A reference from one symbol/file to another.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Reference {
    pub kind: RefKind,
}

#[derive(Debug, Clone)]
pub enum RefKind {
    Import,
}

/// Supported languages for tree-sitter parsing.
enum Lang {
    Python,
    Rust,
}

fn detect_language(path: &Path) -> Option<Lang> {
    match path.extension()?.to_str()? {
        "py" => Some(Lang::Python),
        "rs" => Some(Lang::Rust),
        _ => None,
    }
}

fn create_parser(lang: &Lang) -> Result<Parser, Box<dyn std::error::Error>> {
    let mut parser = Parser::new();
    match lang {
        Lang::Python => parser.set_language(&tree_sitter_python::LANGUAGE.into())?,
        Lang::Rust => parser.set_language(&tree_sitter_rust::LANGUAGE.into())?,
    }
    Ok(parser)
}

/// Scan a directory for source files.
pub fn scan_directory(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    scan_recursive(root, &mut files);
    files.sort();
    files
}

fn scan_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }

        // Skip hidden dirs, target/, __pycache__, .git, node_modules
        if name_str.starts_with('.')
            || name_str == "target"
            || name_str == "__pycache__"
            || name_str == "node_modules"
            || name_str == ".git"
        {
            continue;
        }

        if path.is_dir() {
            scan_recursive(&path, files);
        } else if detect_language(&path).is_some() {
            files.push(path);
        }
    }
}

/// Extract symbols from a single source file.
fn extract_symbols(path: &Path, source: &str, tree: &Tree) -> Vec<Symbol> {
    let lang = match detect_language(path) {
        Some(l) => l,
        None => return Vec::new(),
    };
    let root = tree.root_node();
    let mut symbols = Vec::new();

    match lang {
        Lang::Python => extract_python_symbols(source, root, &mut symbols),
        Lang::Rust => extract_rust_symbols(source, root, &mut symbols),
    }

    symbols
}

fn extract_python_symbols(source: &str, node: Node, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = node_text(source, name_node);
                    let sig = extract_line(source, child.start_position().row);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,

                        line: child.start_position().row + 1,
                        signature: sig,
                    });
                }
            }
            "class_definition" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = node_text(source, name_node);
                    let sig = extract_line(source, child.start_position().row);
                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Class,

                        line: child.start_position().row + 1,
                        signature: sig,
                    });
                    // Extract methods inside class body
                    if let Some(body) = child.child_by_field_name("body") {
                        let mut body_cursor = body.walk();
                        for method in body.children(&mut body_cursor) {
                            if method.kind() == "function_definition" {
                                if let Some(method_name) = method.child_by_field_name("name") {
                                    let mname = node_text(source, method_name);
                                    let msig = extract_line(source, method.start_position().row);
                                    symbols.push(Symbol {
                                        name: format!("{name}.{mname}"),
                                        kind: SymbolKind::Method,

                                        line: method.start_position().row + 1,
                                        signature: msig.trim().to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "import_statement" | "import_from_statement" => {
                let sig = node_text(source, child);
                symbols.push(Symbol {
                    name: sig.clone(),
                    kind: SymbolKind::Import,

                    line: child.start_position().row + 1,
                    signature: sig,
                });
            }
            _ => {
                // Recurse into compound statements
                extract_python_symbols(source, child, symbols);
            }
        }
    }
}

fn extract_rust_symbols(source: &str, node: Node, symbols: &mut Vec<Symbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = node_text(source, name_node);
                    let sig = extract_fn_signature(source, child);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,

                        line: child.start_position().row + 1,
                        signature: sig,
                    });
                }
            }
            "struct_item" | "enum_item" | "trait_item" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = node_text(source, name_node);
                    let sig = extract_line(source, child.start_position().row);
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Class,

                        line: child.start_position().row + 1,
                        signature: sig,
                    });
                }
            }
            "impl_item" => {
                // Extract methods from impl blocks
                let impl_name = child
                    .child_by_field_name("type")
                    .map(|n| node_text(source, n))
                    .unwrap_or_default();
                if let Some(body) = child.child_by_field_name("body") {
                    let mut body_cursor = body.walk();
                    for item in body.children(&mut body_cursor) {
                        if item.kind() == "function_item" {
                            if let Some(name_node) = item.child_by_field_name("name") {
                                let mname = node_text(source, name_node);
                                let msig = extract_fn_signature(source, item);
                                symbols.push(Symbol {
                                    name: format!("{impl_name}::{mname}"),
                                    kind: SymbolKind::Method,

                                    line: item.start_position().row + 1,
                                    signature: msig,
                                });
                            }
                        }
                    }
                }
            }
            "use_declaration" => {
                let sig = node_text(source, child);
                symbols.push(Symbol {
                    name: sig.clone(),
                    kind: SymbolKind::Import,

                    line: child.start_position().row + 1,
                    signature: sig,
                });
            }
            "mod_item" => {
                // Recurse into inline module definitions
                if let Some(body) = child.child_by_field_name("body") {
                    extract_rust_symbols(source, body, symbols);
                }
            }
            _ => {}
        }
    }
}

/// Extract a Rust function signature (up to the opening brace).
fn extract_fn_signature(source: &str, node: Node) -> String {
    let start = node.start_byte();
    let text = &source[start..];
    if let Some(brace) = text.find('{') {
        text[..brace].trim().to_string()
    } else {
        extract_line(source, node.start_position().row)
    }
}

fn node_text(source: &str, node: Node) -> String {
    source[node.start_byte()..node.end_byte()].to_string()
}

fn extract_line(source: &str, row: usize) -> String {
    source.lines().nth(row).unwrap_or("").to_string()
}

/// Build a directed graph of symbols with cross-file references.
/// Nodes are (file_index, symbol_index). Edges are references.
fn build_graph(
    all_symbols: &[(PathBuf, Vec<Symbol>)],
) -> (DiGraph<usize, Reference>, Vec<(PathBuf, Symbol)>) {
    let mut graph = DiGraph::new();
    let mut flat: Vec<(PathBuf, Symbol)> = Vec::new();
    let mut node_indices: Vec<NodeIndex> = Vec::new();
    let mut name_to_nodes: HashMap<String, Vec<usize>> = HashMap::new();

    // Add all symbols as nodes.
    for (path, symbols) in all_symbols {
        for sym in symbols {
            let idx = flat.len();
            flat.push((path.clone(), sym.clone()));
            let node_idx = graph.add_node(idx);
            node_indices.push(node_idx);

            // Index by bare name (strip module prefix for methods).
            let bare = sym.name.rsplit("::").next().unwrap_or(&sym.name);
            let bare = bare.rsplit('.').next().unwrap_or(bare);
            name_to_nodes.entry(bare.to_string()).or_default().push(idx);
        }
    }

    // Add edges: if file A has a symbol name that matches a definition in file B,
    // add an edge from A's symbol to B's definition.
    for (i, (path_i, sym_i)) in flat.iter().enumerate() {
        if sym_i.kind == SymbolKind::Import {
            // Import references: link to any matching definition
            let imported_name = sym_i.name.rsplit(' ').next().unwrap_or(&sym_i.name);
            let imported_name = imported_name.rsplit("::").next().unwrap_or(imported_name);
            if let Some(targets) = name_to_nodes.get(imported_name) {
                for &t in targets {
                    if t != i && flat[t].0 != *path_i {
                        graph.add_edge(
                            node_indices[i],
                            node_indices[t],
                            Reference {
                                kind: RefKind::Import,
                            },
                        );
                    }
                }
            }
        }
    }

    (graph, flat)
}

/// Simple PageRank implementation.
/// Returns scores indexed by node position in `flat`.
fn pagerank(
    graph: &DiGraph<usize, Reference>,
    flat_len: usize,
    focus_files: &[PathBuf],
    flat: &[(PathBuf, Symbol)],
    iterations: usize,
    damping: f64,
) -> Vec<f64> {
    let n = graph.node_count();
    if n == 0 {
        return vec![0.0; flat_len];
    }

    // Personalization vector: boost focus files.
    let mut personalization = vec![1.0 / n as f64; n];
    if !focus_files.is_empty() {
        let mut total = 0.0;
        for (i, (path, _)) in flat.iter().enumerate() {
            if i < n {
                let is_focus = focus_files.iter().any(|f| path.ends_with(f));
                personalization[i] = if is_focus { 3.0 } else { 1.0 };
                total += personalization[i];
            }
        }
        for p in &mut personalization {
            *p /= total;
        }
    }

    let mut scores = vec![1.0 / n as f64; n];

    for _ in 0..iterations {
        let mut new_scores = vec![0.0; n];
        for node_idx in graph.node_indices() {
            let i = node_idx.index();
            let out_degree = graph.edges_directed(node_idx, Direction::Outgoing).count();
            if out_degree > 0 {
                let share = scores[i] / out_degree as f64;
                for edge in graph.edges_directed(node_idx, Direction::Outgoing) {
                    let target = edge.target().index();
                    new_scores[target] += share;
                }
            } else {
                // Dangling node: distribute evenly
                let share = scores[i] / n as f64;
                for s in &mut new_scores {
                    *s += share;
                }
            }
        }

        // Apply damping with personalization.
        for (i, score) in new_scores.iter_mut().enumerate() {
            *score = (1.0 - damping) * personalization[i] + damping * *score;
        }

        scores = new_scores;
    }

    // Map back to flat indices.
    let mut result = vec![0.0; flat_len];
    for (i, &score) in scores.iter().enumerate() {
        if i < flat_len {
            result[i] = score;
        }
    }
    result
}

/// Render the repo map as a readable string within a token budget.
fn render_map(flat: &[(PathBuf, Symbol)], scores: &[f64], root: &Path, budget: usize) -> String {
    // Sort by score descending, skip imports.
    let mut ranked: Vec<(usize, f64)> = scores
        .iter()
        .enumerate()
        .filter(|(i, _)| flat[*i].1.kind != SymbolKind::Import)
        .map(|(i, &s)| (i, s))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Group by file, maintaining rank order.
    let mut file_groups: Vec<(PathBuf, Vec<&Symbol>)> = Vec::new();
    let mut seen_files: HashMap<PathBuf, usize> = HashMap::new();

    for (idx, _score) in &ranked {
        let (path, sym) = &flat[*idx];
        if let Some(&group_idx) = seen_files.get(path) {
            file_groups[group_idx].1.push(sym);
        } else {
            let group_idx = file_groups.len();
            seen_files.insert(path.clone(), group_idx);
            file_groups.push((path.clone(), vec![sym]));
        }
    }

    // Render within budget (approximate: 1 token ≈ 4 chars).
    let char_budget = budget * 4;
    let mut output = String::from("# Repository Map (ranked by relevance)\n");
    let mut chars_used = output.len();

    for (path, symbols) in &file_groups {
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        let header = format!("\n## {}\n", rel_path.display());

        if chars_used + header.len() > char_budget {
            break;
        }
        output.push_str(&header);
        chars_used += header.len();

        for sym in symbols {
            let line = format!("  L{}: {}\n", sym.line, sym.signature);
            if chars_used + line.len() > char_budget {
                break;
            }
            output.push_str(&line);
            chars_used += line.len();
        }
    }

    output
}

type SymbolGraph = DiGraph<usize, Reference>;
type FlatSymbols = Vec<(PathBuf, Symbol)>;
type RepoAnalysis = (PathBuf, Vec<PathBuf>, SymbolGraph, FlatSymbols, Vec<f64>);

fn analyze_repository(
    root: &Path,
    focus: &[PathBuf],
) -> Result<RepoAnalysis, Box<dyn std::error::Error>> {
    let canonical = safety::resolve_existing_directory(root).map_err(|e| e.clone())?;
    let files = scan_directory(&canonical);
    if files.is_empty() {
        return Ok((canonical, files, DiGraph::new(), Vec::new(), Vec::new()));
    }

    let mut all_symbols: Vec<(PathBuf, Vec<Symbol>)> = Vec::new();
    for file in &files {
        let lang = match detect_language(file) {
            Some(l) => l,
            None => continue,
        };
        let mut parser = create_parser(&lang)?;
        let source =
            fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
        if let Some(tree) = parser.parse(&source, None) {
            let symbols = extract_symbols(file, &source, &tree);
            if !symbols.is_empty() {
                all_symbols.push((file.clone(), symbols));
            }
        }
    }

    let (graph, flat) = build_graph(&all_symbols);
    let scores = pagerank(&graph, flat.len(), focus, &flat, 20, 0.85);
    Ok((canonical, files, graph, flat, scores))
}

/// Generate a repo map string for the given path, budget, and focus files.
/// Used by the agent tool registry — same logic as `run` but returns a String.
pub fn generate(
    root: &Path,
    budget: usize,
    focus: &[PathBuf],
) -> Result<String, Box<dyn std::error::Error>> {
    let (canonical, files, _graph, flat, scores) = analyze_repository(root, focus)?;
    if files.is_empty() {
        return Ok(format!(
            "# Repository Map\n\nNo supported source files found in {}",
            canonical.display()
        ));
    }
    Ok(render_map(&flat, &scores, &canonical, budget))
}

/// Run the repomap subcommand.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = PathBuf::from(".");
    let mut budget: usize = defaults::DEFAULT_REPOMAP_BUDGET;
    let mut focus: Vec<PathBuf> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                i += 1;
                path = PathBuf::from(args.get(i).ok_or("--path requires a value")?);
            }
            "--budget" => {
                i += 1;
                budget = args
                    .get(i)
                    .ok_or("--budget requires a value")?
                    .parse()
                    .map_err(|_| "--budget must be a number")?;
            }
            "--focus" => {
                i += 1;
                let val = args.get(i).ok_or("--focus requires a value")?;
                for f in val.split(',') {
                    focus.push(PathBuf::from(f.trim()));
                }
            }
            other => {
                return Err(format!("unknown repomap flag: {other}").into());
            }
        }
        i += 1;
    }

    let (canonical, files, _graph, flat, scores) = analyze_repository(&path, &focus)?;
    if files.is_empty() {
        println!(
            "# Repository Map\n\nNo supported source files found in {}",
            canonical.display()
        );
        return Ok(());
    }

    // Render and print.
    let output = render_map(&flat, &scores, &canonical, budget);
    print!("{output}");

    eprintln!(
        "({} files, {} symbols, {} tokens budget)",
        files.len(),
        flat.len(),
        budget
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_edges_are_not_duplicated_by_unrelated_symbols() {
        let all_symbols = vec![
            (
                PathBuf::from("a.rs"),
                vec![Symbol {
                    name: "use crate::Thing".to_string(),
                    kind: SymbolKind::Import,
                    line: 1,
                    signature: "use crate::Thing;".to_string(),
                }],
            ),
            (
                PathBuf::from("b.rs"),
                vec![Symbol {
                    name: "Thing".to_string(),
                    kind: SymbolKind::Class,
                    line: 1,
                    signature: "struct Thing;".to_string(),
                }],
            ),
            (
                PathBuf::from("c.rs"),
                vec![Symbol {
                    name: "Irrelevant".to_string(),
                    kind: SymbolKind::Function,
                    line: 1,
                    signature: "fn irrelevant() {}".to_string(),
                }],
            ),
        ];

        let (graph, _flat) = build_graph(&all_symbols);
        assert_eq!(graph.edge_count(), 1);
    }
}
