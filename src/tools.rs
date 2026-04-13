#![allow(clippy::unnecessary_literal_bound)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use glob::Pattern;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::mcp_client::SharedMcpClient;

pub type ToolResult = Result<String, String>;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
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
                "command": {"type": "string"}
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let command = args
            .get("command")
            .and_then(Value::as_str)
            .ok_or_else(|| "bash requires string field: command".to_string())?;
        let blocked = ["rm -rf /", "sudo ", "mkfs", "dd if="];
        if blocked.iter().any(|needle| command.contains(needle)) {
            return Err("refusing potentially destructive command".to_string());
        }

        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| format!("failed to run bash command: {e}"))?;

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
            "properties":{"path":{"type":"string"}},
            "required":["path"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "read_file requires string field: path".to_string())?;
        fs::read_to_string(path).map_err(|e| format!("failed reading {path}: {e}"))
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
        let path_obj = Path::new(path);
        if let Some(parent) = path_obj.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed creating parent dir {}: {e}", parent.display()))?;
        }
        fs::write(path_obj, content).map_err(|e| format!("failed writing {path}: {e}"))?;
        Ok(format!("written: {path}"))
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
        let path_obj = Path::new(path);
        let before =
            fs::read_to_string(path_obj).map_err(|e| format!("failed reading {path}: {e}"))?;
        let ops = crate::hashline::parse_edits(edits);
        if ops.is_empty() {
            return Err("no valid hashline operations parsed".to_string());
        }
        let after = crate::hashline::apply_edits(path_obj, &ops).map_err(|e| e.to_string())?;
        fs::write(path_obj, &after).map_err(|e| format!("failed writing {path}: {e}"))?;
        Ok(format!(
            "edited: {path}, lines_before={}, lines_after={}, ops={}",
            before.lines().count(),
            after.lines().count(),
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

        let mut out = Vec::new();
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
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

    async fn execute(&self, args: Value) -> ToolResult {
        let root = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "list_files requires string field: path".to_string())?;
        let glob = args.get("glob").and_then(Value::as_str);
        let glob_pattern = compile_glob(glob)?;

        let mut entries = Vec::new();
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || !matches_glob(path, glob_pattern.as_ref()) {
                continue;
            }
            let rel = path
                .strip_prefix(root)
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
                "budget":{"type":"integer","default":4096},
                "focus":{"type":"string","description":"Comma-separated focus files"}
            },
            "required":["path"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let root = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "repomap requires string field: path".to_string())?;
        #[allow(clippy::cast_possible_truncation)]
        let budget = args.get("budget").and_then(Value::as_u64).unwrap_or(4096) as usize;
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
                "context":{"type":"string"}
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
            return Err("dispatch level must be 2 or 3".to_string());
        }
        let task = args
            .get("task")
            .and_then(Value::as_str)
            .ok_or_else(|| "dispatch requires string field: task".to_string())?;
        let context = args.get("context").and_then(Value::as_str).unwrap_or("");
        let payload = json!({
            "task": task,
            "context": context,
            "constraints": []
        });
        let input = serde_json::to_string(&payload).map_err(|e| format!("payload error: {e}"))?;
        crate::dispatch::run_capture(level, &input).map_err(|e| e.to_string())
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
}

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
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn definitions(&self) -> Vec<Value> {
        self.tools
            .iter()
            .map(|tool| tool_definition(tool.as_ref()))
            .collect()
    }

    pub async fn execute(&self, name: &str, args: Value) -> ToolResult {
        if let Some(tool) = self.tools.iter().find(|tool| tool.name() == name) {
            tool.execute(args).await
        } else {
            Err(format!("unknown tool: {name}"))
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

#[allow(dead_code)]
fn _normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
