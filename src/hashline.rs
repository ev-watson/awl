#![allow(clippy::format_push_string, clippy::needless_range_loop)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Alphabet for 2-char hash tags (16 chars → 16² = 256 buckets).
/// Matches oh-my-pi's convention: visually distinct, no digits, no vowels.
const ALPHABET: &[u8; 16] = b"ZPMQVRWSNKTXJBYH";

/// Hash a single line's content to a 2-character tag.
/// Uses a simple FNV-1a-style hash (fast, no crate dependency).
fn hash_line(content: &str) -> String {
    let normalized = content.trim_end();
    let mut h: u32 = 2_166_136_261;
    for byte in normalized.bytes() {
        h ^= u32::from(byte);
        h = h.wrapping_mul(16_777_619);
    }
    let hi = ALPHABET[((h >> 4) & 0x0F) as usize];
    let lo = ALPHABET[(h & 0x0F) as usize];
    String::from_utf8(vec![hi, lo]).unwrap()
}

/// A single line with its number and hash.
#[derive(Debug, Clone)]
pub struct HashedLine {
    pub number: usize,
    pub hash: String,
    pub content: String,
}

/// Read a file and return hashline-formatted output.
/// Format: `LINE:HASH|content`
pub fn format_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let lines = hash_lines(&text);
    let mut out = String::new();
    for line in &lines {
        out.push_str(&format!("{}:{}|{}\n", line.number, line.hash, line.content));
    }
    Ok(out)
}

/// Hash all lines in a string.
fn hash_lines(text: &str) -> Vec<HashedLine> {
    text.lines()
        .enumerate()
        .map(|(i, content)| HashedLine {
            number: i + 1,
            hash: hash_line(content),
            content: content.to_string(),
        })
        .collect()
}

/// Build a lookup from "LINE:HASH" to line index for validation.
fn build_lookup(lines: &[HashedLine]) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (idx, line) in lines.iter().enumerate() {
        let key = format!("{}:{}", line.number, line.hash);
        map.insert(key, idx);
    }
    map
}

/// An edit operation parsed from model output.
#[derive(Debug, Clone)]
pub enum EditOp {
    /// Replace a single line identified by LINE:HASH with new content.
    ReplaceLine { anchor: String, new_content: String },
    /// Replace a range from start LINE:HASH through end LINE:HASH.
    ReplaceRange {
        start: String,
        end: String,
        new_content: String,
    },
    /// Insert new content after the line identified by LINE:HASH.
    InsertAfter { anchor: String, new_content: String },
    /// Delete a single line identified by LINE:HASH.
    DeleteLine { anchor: String },
    /// Delete a range from start through end.
    DeleteRange { start: String, end: String },
}

/// Parse edit instructions from model output.
///
/// Supported formats:
///   replace LINE:HASH with <content>
///   replace LINE:HASH through LINE:HASH with <content>
///   insert after LINE:HASH <content>
///   delete LINE:HASH
///   delete LINE:HASH through LINE:HASH
pub fn parse_edits(input: &str) -> Vec<EditOp> {
    let mut ops = Vec::new();
    let mut lines = input.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_lowercase();

        if lower.starts_with("replace ") {
            let rest = &trimmed[8..];
            if let Some(through_pos) = find_keyword(rest, " through ") {
                let start = rest[..through_pos].trim().to_string();
                let after_through = &rest[through_pos + 9..];
                if let Some(with_pos) = find_keyword(after_through, " with ") {
                    let end = after_through[..with_pos].trim().to_string();
                    let new_content = collect_content(&after_through[with_pos + 6..], &mut lines);
                    ops.push(EditOp::ReplaceRange {
                        start,
                        end,
                        new_content,
                    });
                }
            } else if let Some(with_pos) = find_keyword(rest, " with ") {
                let anchor = rest[..with_pos].trim().to_string();
                let new_content = collect_content(&rest[with_pos + 6..], &mut lines);
                ops.push(EditOp::ReplaceLine {
                    anchor,
                    new_content,
                });
            }
        } else if lower.starts_with("insert after ") {
            let rest = &trimmed[13..];
            if let Some(space_pos) = rest.find(' ') {
                let anchor = rest[..space_pos].trim().to_string();
                let new_content = collect_content(&rest[space_pos + 1..], &mut lines);
                ops.push(EditOp::InsertAfter {
                    anchor,
                    new_content,
                });
            }
        } else if lower.starts_with("delete ") {
            let rest = &trimmed[7..];
            if let Some(through_pos) = find_keyword(rest, " through ") {
                let start = rest[..through_pos].trim().to_string();
                let end = rest[through_pos + 9..].trim().to_string();
                ops.push(EditOp::DeleteRange { start, end });
            } else {
                ops.push(EditOp::DeleteLine {
                    anchor: rest.trim().to_string(),
                });
            }
        }
    }

    ops
}

/// Case-insensitive keyword search in a string.
fn find_keyword(haystack: &str, keyword: &str) -> Option<usize> {
    let lower = haystack.to_lowercase();
    lower.find(&keyword.to_lowercase())
}

/// Collect multi-line content. If the first line is just the start,
/// subsequent indented or non-command lines are part of the content.
fn collect_content<'a>(
    first: &str,
    lines: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> String {
    let mut content = first.to_string();
    while let Some(next) = lines.peek() {
        let trimmed = next.trim().to_lowercase();
        if trimmed.starts_with("replace ")
            || trimmed.starts_with("insert ")
            || trimmed.starts_with("delete ")
            || trimmed.is_empty()
        {
            break;
        }
        content.push('\n');
        content.push_str(lines.next().unwrap());
    }
    content
}

/// Apply a sequence of edit operations to a file.
/// Validates hashes against current file state before applying.
pub fn apply_edits(path: &Path, ops: &[EditOp]) -> Result<String, Box<dyn std::error::Error>> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let hashed = hash_lines(&text);
    let lookup = build_lookup(&hashed);

    let mut lines: Vec<Option<String>> = hashed.iter().map(|l| Some(l.content.clone())).collect();
    let mut insertions: Vec<(usize, String)> = Vec::new();

    for op in ops {
        match op {
            EditOp::ReplaceLine {
                anchor,
                new_content,
            } => {
                let idx = resolve_anchor(&lookup, anchor)?;
                lines[idx] = Some(new_content.clone());
            }
            EditOp::ReplaceRange {
                start,
                end,
                new_content,
            } => {
                let start_idx = resolve_anchor(&lookup, start)?;
                let end_idx = resolve_anchor(&lookup, end)?;
                if end_idx < start_idx {
                    return Err(format!("range end {end} is before start {start}").into());
                }
                for i in start_idx..=end_idx {
                    lines[i] = None;
                }
                lines[start_idx] = Some(new_content.clone());
            }
            EditOp::InsertAfter {
                anchor,
                new_content,
            } => {
                let idx = resolve_anchor(&lookup, anchor)?;
                insertions.push((idx, new_content.clone()));
            }
            EditOp::DeleteLine { anchor } => {
                let idx = resolve_anchor(&lookup, anchor)?;
                lines[idx] = None;
            }
            EditOp::DeleteRange { start, end } => {
                let start_idx = resolve_anchor(&lookup, start)?;
                let end_idx = resolve_anchor(&lookup, end)?;
                if end_idx < start_idx {
                    return Err(format!("range end {end} is before start {start}").into());
                }
                for i in start_idx..=end_idx {
                    lines[i] = None;
                }
            }
        }
    }

    // Group insertions by position.
    let mut insertion_map: HashMap<usize, Vec<String>> = HashMap::new();
    for (idx, content) in insertions {
        insertion_map.entry(idx).or_default().push(content);
    }

    // Build the result.
    let mut result_lines: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(content) = line {
            for subline in content.lines() {
                result_lines.push(subline.to_string());
            }
        }
        if let Some(inserts) = insertion_map.get(&i) {
            for ins_content in inserts {
                for subline in ins_content.lines() {
                    result_lines.push(subline.to_string());
                }
            }
        }
    }

    let result = result_lines.join("\n");
    Ok(result)
}

/// Resolve an anchor like "5:PZ" to a line index. Validates the hash matches.
fn resolve_anchor(
    lookup: &HashMap<String, usize>,
    anchor: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    lookup.get(anchor).copied().ok_or_else(|| {
        format!("hashline anchor '{anchor}' not found — file may have changed since last read")
            .into()
    })
}

/// Run the hashline subcommand.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        return Err("hashline requires a subcommand: read, apply".into());
    }

    match args[0].as_str() {
        "read" => {
            let path = args.get(1).ok_or("hashline read requires a file path")?;
            let output = format_file(Path::new(path))?;
            print!("{output}");
            Ok(())
        }
        "apply" => {
            let path = args.get(1).ok_or("hashline apply requires a file path")?;
            let mut input = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
            let ops = parse_edits(&input);
            if ops.is_empty() {
                return Err("no valid edit operations found in stdin".into());
            }
            let result = apply_edits(Path::new(path), &ops)?;
            fs::write(path, &result).map_err(|e| format!("cannot write {path}: {e}"))?;
            println!("applied {} edit(s) to {path}", ops.len());
            Ok(())
        }
        other => Err(format!("unknown hashline subcommand: {other}").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        let h1 = hash_line("fn main() {");
        let h2 = hash_line("fn main() {");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 2);
    }

    #[test]
    fn hash_differs_for_different_content() {
        let h1 = hash_line("fn main() {");
        let h2 = hash_line("fn foo() {");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_ignores_trailing_whitespace() {
        let h1 = hash_line("fn main() {");
        let h2 = hash_line("fn main() {   ");
        assert_eq!(h1, h2);
    }

    #[test]
    fn format_produces_hashline_output() {
        let text = "fn main() {\n    println!(\"hello\");\n}\n";
        let lines = hash_lines(text);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].number, 1);
        assert_eq!(lines[0].content, "fn main() {");
    }

    #[test]
    fn parse_replace_line() {
        let input = "replace 2:XY with     println!(\"world\");";
        let ops = parse_edits(input);
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], EditOp::ReplaceLine { anchor, .. } if anchor == "2:XY"));
    }

    #[test]
    fn parse_delete_line() {
        let input = "delete 3:AB";
        let ops = parse_edits(input);
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], EditOp::DeleteLine { anchor } if anchor == "3:AB"));
    }

    #[test]
    fn parse_insert_after() {
        let input = "insert after 1:CD     let x = 42;";
        let ops = parse_edits(input);
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], EditOp::InsertAfter { anchor, .. } if anchor == "1:CD"));
    }

    #[test]
    fn parse_delete_range() {
        let input = "delete 2:AB through 4:CD";
        let ops = parse_edits(input);
        assert_eq!(ops.len(), 1);
        assert!(
            matches!(&ops[0], EditOp::DeleteRange { start, end } if start == "2:AB" && end == "4:CD")
        );
    }
}
