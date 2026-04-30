#![allow(clippy::unnecessary_literal_bound)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use glob::Pattern;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::defaults;
use crate::mcp_client::SharedMcpClient;
use crate::safety;

pub type ToolResult = Result<String, String>;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn cacheable(&self) -> bool {
        false
    }
    async fn execute(&self, args: Value) -> ToolResult;
}

pub fn tool_definition(tool: &dyn Tool) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name(),
            "description": tool.description(),
            "parameters": tool.parameters_schema()
        }
    })
}

struct BashTool;
struct ReadFileTool;
struct WriteFileTool;
struct EditFileTool;
struct SearchFilesTool;
struct ListFilesTool;
struct RepoMapTool;
struct DispatchTool;

pub struct McpToolProxy {
    tool_name: String,
    tool_description: String,
    schema: Value,
    client: SharedMcpClient,
}

struct ToolCache {
    entries: HashMap<(String, u64), CacheEntry>,
    max_entries: usize,
}

struct CacheEntry {
    result: String,
    hits: u32,
}

struct FileSnapshot {
    path: PathBuf,
    previous_contents: Option<Vec<u8>>,
}

impl ToolCache {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
        }
    }

    fn get(&mut self, key: &(String, u64)) -> Option<String> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.hits += 1;
            Some(entry.result.clone())
        } else {
            None
        }
    }

    fn insert(&mut self, key: (String, u64), result: String) {
        if self.entries.len() >= self.max_entries {
            if let Some(evict_key) = self
                .entries
                .iter()
                .min_by_key(|(_, v)| v.hits)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&evict_key);
            }
        }
        self.entries.insert(key, CacheEntry { result, hits: 1 });
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

fn hash_args(args: &Value) -> u64 {
    let serialized = serde_json::to_string(args).unwrap_or_default();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in serialized.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn capture_snapshot(path: &str) -> Result<FileSnapshot, String> {
    let resolved = safety::resolve_path_for_write(Path::new(path))?;
    let previous_contents = match fs::read(&resolved) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "failed to snapshot {} before edit: {error}",
                resolved.display()
            ));
        }
    };
    Ok(FileSnapshot {
        path: resolved,
        previous_contents,
    })
}

fn restore_snapshot(snapshot: FileSnapshot) -> Result<String, String> {
    if let Some(contents) = snapshot.previous_contents {
        fs::write(&snapshot.path, contents).map_err(|e| {
            format!(
                "undo failed while restoring {}: {e}",
                snapshot.path.display()
            )
        })?;
        Ok(format!(
            "Restored {} to previous state.",
            snapshot.path.display()
        ))
    } else {
        if snapshot.path.exists() {
            fs::remove_file(&snapshot.path).map_err(|e| {
                format!(
                    "undo failed while removing newly created {}: {e}",
                    snapshot.path.display()
                )
            })?;
        }
        Ok(format!(
            "Removed {} because it did not exist before the edit.",
            snapshot.path.display()
        ))
    }
}

impl McpToolProxy {
    pub fn new(name: String, description: String, schema: Value, client: SharedMcpClient) -> Self {
        Self {
            tool_name: name,
            tool_description: description,
            schema,
            client,
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Run a shell command and return combined stdout/stderr."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string"},
                "timeout_ms": {"type": "integer", "description": "Timeout in milliseconds (default: 120000)"}
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let command = args
            .get("command")
            .and_then(Value::as_str)
            .ok_or_else(|| "bash requires string field: command".to_string())?;
        safety::validate_shell_command(command)?;
        let workspace = safety::workspace_root()?;

        let timeout_ms: u64 = args
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(120_000);
        let mut child = std::process::Command::new("bash")
            .arg("-lc")
            .current_dir(workspace)
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to run bash command: {e}"))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let output = loop {
            match child.try_wait().map_err(|e| format!("wait failed: {e}"))? {
                Some(status) => {
                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();
                    if let Some(mut s) = child.stdout.take() {
                        std::io::Read::read_to_end(&mut s, &mut stdout).ok();
                    }
                    if let Some(mut s) = child.stderr.take() {
                        std::io::Read::read_to_end(&mut s, &mut stderr).ok();
                    }
                    break std::process::Output {
                        status,
                        stdout,
                        stderr,
                    };
                }
                None if std::time::Instant::now() >= deadline => {
                    let _ = child.kill();
                    return Err(format!("bash command timed out after {timeout_ms}ms"));
                }
                None => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        };

        let mut combined = String::new();
        combined.push_str(&String::from_utf8_lossy(&output.stdout));
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(truncate_text(&combined, 8_000))
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read file contents as text."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "path":{"type":"string"},
                "offset":{"type":"integer","description":"Line number to start from (1-based, default: 1)"},
                "limit":{"type":"integer","description":"Max lines to return (default: all)"}
            },
            "required":["path"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "read_file requires string field: path".to_string())?;
        let resolved = safety::resolve_existing_path(Path::new(path))?;
        let raw = fs::read_to_string(&resolved)
            .map_err(|e| format!("failed reading {}: {e}", resolved.display()))?;
        #[allow(clippy::cast_possible_truncation)]
        let offset = args
            .get("offset")
            .and_then(Value::as_u64)
            .map_or(0, |v| v.saturating_sub(1) as usize);
        #[allow(clippy::cast_possible_truncation)]
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|v| v as usize);
        if offset > 0 || limit.is_some() {
            let lines: Vec<&str> = raw.lines().skip(offset).collect();
            let taken = match limit {
                Some(n) => &lines[..n.min(lines.len())],
                None => &lines,
            };
            Ok(taken.join("\n"))
        } else {
            Ok(raw)
        }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write text content to a file, creating parent dirs as needed."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "path":{"type":"string"},
                "content":{"type":"string"}
            },
            "required":["path","content"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "write_file requires string field: path".to_string())?;
        let content = args
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| "write_file requires string field: content".to_string())?;
        let path_obj = safety::resolve_path_for_write(Path::new(path))?;
        if let Some(parent) = path_obj.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed creating parent dir {}: {e}", parent.display()))?;
        }
        fs::write(&path_obj, content)
            .map_err(|e| format!("failed writing {}: {e}", path_obj.display()))?;
        Ok(format!("written: {}", path_obj.display()))
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Apply hashline edit operations to a file."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "path":{"type":"string"},
                "edits":{"type":"string","description":"Hashline edit instructions"}
            },
            "required":["path","edits"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "edit_file requires string field: path".to_string())?;
        let edits = args
            .get("edits")
            .and_then(Value::as_str)
            .ok_or_else(|| "edit_file requires string field: edits".to_string())?;
        let path_obj = safety::resolve_existing_path(Path::new(path))?;
        let before =
            fs::read_to_string(&path_obj).map_err(|e| format!("failed reading {path}: {e}"))?;
        let ops = crate::hashline::parse_edits(edits);
        if ops.is_empty() {
            return Err(
                "no valid hashline operations parsed from edits field. \
                 Expected format: REPLACE LINE:HASH newcontent / DELETE LINE:HASH / INSERT AFTER LINE:HASH newcontent"
                    .to_string(),
            );
        }
        let after = crate::hashline::apply_edits(&path_obj, &ops).map_err(|e| e.to_string())?;
        fs::write(&path_obj, &after).map_err(|e| format!("failed writing {path}: {e}"))?;
        Ok(format!(
            "edited: {}, lines_before={}, lines_after={}, ops={}",
            path_obj.display(),
            before.split('\n').count(),
            after.split('\n').count(),
            ops.len()
        ))
    }
}

#[async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }

    fn description(&self) -> &str {
        "Search for a text pattern in files under a directory."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "path":{"type":"string"},
                "pattern":{"type":"string"},
                "glob":{"type":"string","description":"Optional file glob (example: *.rs)"}
            },
            "required":["path","pattern"]
        })
    }

    fn cacheable(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let root = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "search_files requires string field: path".to_string())?;
        let pattern = args
            .get("pattern")
            .and_then(Value::as_str)
            .ok_or_else(|| "search_files requires string field: pattern".to_string())?;
        let glob = args.get("glob").and_then(Value::as_str);
        let glob_pattern = compile_glob(glob)?;
        let resolved_root = safety::resolve_existing_directory(Path::new(root))?;

        let mut out = Vec::new();
        for entry in WalkDir::new(&resolved_root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() || !matches_glob(path, glob_pattern.as_ref()) {
                continue;
            }
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            for (idx, line) in content.lines().enumerate() {
                if line.contains(pattern) {
                    out.push(format!("{}:{}: {}", path.display(), idx + 1, line));
                    if out.len() >= 100 {
                        break;
                    }
                }
            }
            if out.len() >= 100 {
                break;
            }
        }
        if out.is_empty() {
            Ok("no matches".to_string())
        } else {
            Ok(out.join("\n"))
        }
    }
}

#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files under a directory, optionally filtered by glob."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "path":{"type":"string"},
                "glob":{"type":"string"}
            },
            "required":["path"]
        })
    }

    fn cacheable(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let root = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "list_files requires string field: path".to_string())?;
        let glob = args.get("glob").and_then(Value::as_str);
        let glob_pattern = compile_glob(glob)?;
        let resolved_root = safety::resolve_existing_directory(Path::new(root))?;

        let mut entries = Vec::new();
        for entry in WalkDir::new(&resolved_root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() || !matches_glob(path, glob_pattern.as_ref()) {
                continue;
            }
            let rel = path
                .strip_prefix(&resolved_root)
                .map_or_else(|_| path.display().to_string(), |p| p.display().to_string());
            entries.push(rel);
            if entries.len() >= 500 {
                break;
            }
        }
        if entries.is_empty() {
            Ok("no files".to_string())
        } else {
            Ok(entries.join("\n"))
        }
    }
}

#[async_trait]
impl Tool for RepoMapTool {
    fn name(&self) -> &str {
        "repomap"
    }

    fn description(&self) -> &str {
        "Generate a lightweight repository map for source files."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "path":{"type":"string"},
                "budget":{"type":"integer","default":defaults::DEFAULT_REPOMAP_BUDGET},
                "focus":{"type":"string","description":"Comma-separated focus files"}
            },
            "required":["path"]
        })
    }

    fn cacheable(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let root = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "repomap requires string field: path".to_string())?;
        #[allow(clippy::cast_possible_truncation)]
        let budget = args
            .get("budget")
            .and_then(Value::as_u64)
            .unwrap_or(defaults::DEFAULT_REPOMAP_BUDGET as u64) as usize;
        let focus: Vec<PathBuf> = args
            .get("focus")
            .and_then(Value::as_str)
            .map(|s| s.split(',').map(|f| PathBuf::from(f.trim())).collect())
            .unwrap_or_default();
        crate::repomap::generate(Path::new(root), budget, &focus).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl Tool for DispatchTool {
    fn name(&self) -> &str {
        "dispatch"
    }

    fn description(&self) -> &str {
        "Delegate a subtask to level 2 (implementation) or level 3 (verification)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "level":{"type":"integer","enum":[2,3]},
                "task":{"type":"string"},
                "context":{"type":"string"},
                "constraints":{"type":"array","items":{"type":"string"}},
                "target_path":{"type":"string"},
                "target_files":{"type":"array","items":{"type":"string"}},
                "context_paths":{"type":"array","items":{"type":"string"}},
                "verify_command":{"type":"string"},
                "apply":{"type":"boolean"},
                "max_attempts":{"type":"integer"},
                "max_return_chars":{"type":"integer"},
                "auto_repomap":{"type":"boolean"},
                "repomap_focus":{"type":"array","items":{"type":"string"}},
                "repomap_budget":{"type":"integer"}
            },
            "required":["level","task"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let level_u64 = args
            .get("level")
            .and_then(Value::as_u64)
            .ok_or_else(|| "dispatch requires integer field: level".to_string())?;
        let level = u8::try_from(level_u64).map_err(|_| "invalid level value".to_string())?;
        if level != 2 && level != 3 {
            return Err(format!(
                "dispatch level must be 2 or 3 (got {level}). \
                 Level 2 = 7B implementation, level 3 = 3B verification"
            ));
        }
        let task = args
            .get("task")
            .and_then(Value::as_str)
            .ok_or_else(|| "dispatch requires string field: task".to_string())?;
        let context = args.get("context").and_then(Value::as_str).unwrap_or("");
        let constraints = optional_string_array(&args, "constraints")?;
        let target_path = args
            .get("target_path")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let target_files = optional_string_array(&args, "target_files")?;
        let context_paths = optional_string_array(&args, "context_paths")?;
        let verify_command = args
            .get("verify_command")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let apply = args.get("apply").and_then(Value::as_bool).unwrap_or(false);
        let max_attempts = optional_usize(&args, "max_attempts")?;
        let max_return_chars = optional_usize(&args, "max_return_chars")?;
        let auto_repomap = args
            .get("auto_repomap")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let repomap_focus = optional_string_array(&args, "repomap_focus")?;
        let repomap_budget = optional_usize(&args, "repomap_budget")?;
        let payload = json!({
            "task": task,
            "context": context,
            "constraints": constraints,
            "target_path": target_path,
            "target_files": target_files,
            "context_paths": context_paths,
            "verify_command": verify_command,
            "apply": apply,
            "max_attempts": max_attempts,
            "max_return_chars": max_return_chars,
            "auto_repomap": auto_repomap,
            "repomap_focus": repomap_focus,
            "repomap_budget": repomap_budget
        });
        let input = serde_json::to_string(&payload).map_err(|e| format!("payload error: {e}"))?;
        let mut options = crate::dispatch::DispatchOptions::new(level);
        options.apply = apply;
        options.verify_command = verify_command;
        options.target_path = target_path;
        options.max_attempts = max_attempts;
        options.max_return_chars = max_return_chars;
        options.auto_repomap = auto_repomap;
        options.repomap_focus = repomap_focus;
        options.repomap_budget = repomap_budget;
        crate::dispatch::run_capture(&options, &input).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl Tool for McpToolProxy {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, args: Value) -> ToolResult {
        // Strip the "server_name::" prefix — MCP servers expect bare tool names.
        let bare_name = self
            .tool_name
            .rsplit("::")
            .next()
            .unwrap_or(&self.tool_name);
        self.client
            .call_tool(bare_name, args)
            .await
            .map_err(|e| e.to_string())
    }
}

pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
    cache: Mutex<ToolCache>,
    snapshots: Mutex<Vec<FileSnapshot>>,
}

const MUTATING_TOOLS: &[&str] = &["write_file", "edit_file", "bash"];

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: vec![
                Arc::new(BashTool),
                Arc::new(ReadFileTool),
                Arc::new(WriteFileTool),
                Arc::new(EditFileTool),
                Arc::new(SearchFilesTool),
                Arc::new(ListFilesTool),
                Arc::new(RepoMapTool),
                Arc::new(DispatchTool),
            ],
            cache: Mutex::new(ToolCache::new(64)),
            snapshots: Mutex::new(Vec::new()),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn definitions(&self) -> Vec<Value> {
        let mut defs: Vec<Value> = self
            .tools
            .iter()
            .map(|tool| tool_definition(tool.as_ref()))
            .collect();
        defs.push(json!({
            "type": "function",
            "function": {
                "name": "undo_edit",
                "description": "Revert the last file write or edit. Restores prior file contents and removes files that were created by the reverted write.",
                "parameters": {"type": "object", "properties": {}}
            }
        }));
        defs
    }

    pub async fn execute(&self, name: &str, args: Value) -> ToolResult {
        // Handle undo_edit before tool lookup — it operates on the snapshot stack,
        // not a registered tool implementation.
        if name == "undo_edit" {
            return match self.snapshots.lock() {
                Ok(mut snaps) => {
                    if let Some(snapshot) = snaps.pop() {
                        let message = restore_snapshot(snapshot)?;
                        if let Ok(mut cache) = self.cache.lock() {
                            cache.clear();
                        }
                        Ok(message)
                    } else {
                        Err("No snapshots available to undo.".to_string())
                    }
                }
                Err(e) => Err(format!("snapshot lock failed: {e}")),
            };
        }

        let tool = self
            .tools
            .iter()
            .find(|tool| tool.name() == name)
            .ok_or_else(|| format!("unknown tool: {name}"))?;

        // Invalidate cache when a mutating tool runs — even on failure,
        // partial writes may have changed file state.
        if MUTATING_TOOLS.contains(&name) {
            // Snapshot file before write/edit for undo support.
            if name == "write_file" || name == "edit_file" {
                if let Some(path) = args.get("path").and_then(Value::as_str) {
                    let snapshot = capture_snapshot(path)?;
                    if let Ok(mut snaps) = self.snapshots.lock() {
                        snaps.push(snapshot);
                        let len = snaps.len();
                        if len > 20 {
                            snaps.drain(..len - 20);
                        }
                    }
                }
            }
            if let Ok(mut cache) = self.cache.lock() {
                cache.clear();
            }
            return tool.execute(args).await;
        }

        if tool.cacheable() {
            let key = (name.to_string(), hash_args(&args));
            if let Ok(mut cache) = self.cache.lock() {
                if let Some(cached) = cache.get(&key) {
                    return Ok(cached);
                }
            }
            let result = tool.execute(args).await?;
            if let Ok(mut cache) = self.cache.lock() {
                cache.insert(key, result.clone());
            }
            Ok(result)
        } else {
            tool.execute(args).await
        }
    }
}

fn truncate_text(input: &str, limit: usize) -> String {
    if input.len() <= limit {
        return input.to_string();
    }
    let mut out = input.chars().take(limit).collect::<String>();
    out.push_str("\n\n[truncated]");
    out
}

fn optional_string_array(args: &Value, key: &str) -> Result<Vec<String>, String> {
    let Some(value) = args.get(key) else {
        return Ok(Vec::new());
    };
    let Some(values) = value.as_array() else {
        return Err(format!("{key} must be an array of strings"));
    };
    values
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("{key} must be an array of strings"))
        })
        .collect()
}

fn optional_usize(args: &Value, key: &str) -> Result<Option<usize>, String> {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|value| usize::try_from(value).map_err(|_| format!("{key} is too large")))
        .transpose()
}

fn compile_glob(raw: Option<&str>) -> Result<Option<Pattern>, String> {
    if let Some(pattern) = raw {
        let compiled = Pattern::new(pattern).map_err(|e| format!("invalid glob {pattern}: {e}"))?;
        Ok(Some(compiled))
    } else {
        Ok(None)
    }
}

fn matches_glob(path: &Path, glob_pattern: Option<&Pattern>) -> bool {
    match glob_pattern {
        Some(glob) => {
            let file_name = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().to_string());
            glob.matches(&file_name) || glob.matches_path(path)
        }
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock drift")
            .as_nanos();
        std::env::temp_dir().join(format!("awl-tools-{name}-{nanos}.txt"))
    }

    #[test]
    fn test_cache_hit() {
        let mut cache = ToolCache::new(2);
        let key = ("read_file".to_string(), 42);
        cache.insert(key.clone(), "hello".to_string());

        assert_eq!(cache.get(&key).as_deref(), Some("hello"));
        assert_eq!(cache.entries.get(&key).map(|entry| entry.hits), Some(2));
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = ToolCache::new(2);
        let key_a = ("a".to_string(), 1);
        let key_b = ("b".to_string(), 2);
        let key_c = ("c".to_string(), 3);

        cache.insert(key_a.clone(), "A".to_string());
        cache.insert(key_b.clone(), "B".to_string());
        let _ = cache.get(&key_b);
        cache.insert(key_c.clone(), "C".to_string());

        assert!(!cache.entries.contains_key(&key_a));
        assert!(cache.entries.contains_key(&key_b));
        assert!(cache.entries.contains_key(&key_c));
    }

    #[test]
    fn test_hash_args_deterministic() {
        let args = json!({"path":"src/main.rs","budget":defaults::DEFAULT_REPOMAP_BUDGET});
        assert_eq!(hash_args(&args), hash_args(&args));
    }

    #[test]
    fn test_restore_snapshot_rewrites_previous_contents() {
        let path = temp_file_path("restore-existing");
        fs::write(&path, "after").expect("write file");

        let message = restore_snapshot(FileSnapshot {
            path: path.clone(),
            previous_contents: Some(b"before".to_vec()),
        })
        .expect("restore existing snapshot");

        assert_eq!(
            fs::read_to_string(&path).expect("read restored file"),
            "before"
        );
        assert!(message.contains("Restored"));

        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn test_restore_snapshot_removes_new_file() {
        let path = temp_file_path("restore-new");
        fs::write(&path, "new").expect("write file");

        let message = restore_snapshot(FileSnapshot {
            path: path.clone(),
            previous_contents: None,
        })
        .expect("restore missing snapshot");

        assert!(!path.exists());
        assert!(message.contains("did not exist before"));
    }
}
