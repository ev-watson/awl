use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::agent::{self, AgentConfig};
use crate::defaults;
use crate::phases::PhaseState;
use crate::session::Session;

const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[
    "2024-11-05",
    "2025-03-26",
    "2025-06-18",
    LATEST_PROTOCOL_VERSION,
];

#[derive(Debug)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl JsonRpcError {
    fn method_not_found() -> Self {
        Self {
            code: -32601,
            message: "method not found".to_string(),
        }
    }

    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
        }
    }
}

fn server_tool_definitions() -> Vec<Value> {
    let agent_model = defaults::configured_agent_model();
    let mut tools = vec![
        json!({
            "name": "awl_health",
            "description": "Report whether the awl MCP server is alive. This is a lightweight check and does not contact Ollama.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }),
        json!({
            "name": "awl_dispatch",
            "description": "Delegate a coding task to a local Ollama model. Level 2 (7B) for implementation, level 3 (3B) for verification/lint.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "level": {"type": "integer", "enum": [2, 3], "description": "Model tier: 2=implementation (7B), 3=verification (3B)"},
                    "task": {"type": "string", "description": "Task description"},
                    "context": {"type": "string", "description": "Optional code context"},
                    "constraints": {"type": "array", "items": {"type": "string"}, "description": "Optional hard constraints for the worker"},
                    "target_path": {"type": "string", "description": "Single file Awl may write when apply=true"},
                    "target_files": {"type": "array", "items": {"type": "string"}, "description": "Files in scope for the task"},
                    "context_paths": {"type": "array", "items": {"type": "string"}, "description": "Files Awl should read locally and inject into the local model prompt"},
                    "verify_command": {"type": "string", "description": "Command to run after applying changes; failed checks roll back writes"},
                    "apply": {"type": "boolean", "description": "When true, write the generated code to target_path and optionally verify"},
                    "max_attempts": {"type": "integer", "description": "Maximum local apply/verify attempts, capped at 5"},
                    "max_return_chars": {"type": "integer", "description": "Maximum characters of generated code or verifier output returned to the host"},
                    "auto_repomap": {"type": "boolean", "description": "When true, inject a small local repository map into the worker prompt"},
                    "repomap_focus": {"type": "array", "items": {"type": "string"}, "description": "Optional focus files for auto_repomap"},
                    "repomap_budget": {"type": "integer", "description": "Token budget for auto_repomap"}
                },
                "required": ["level", "task"]
            }
        }),
        json!({
            "name": "awl_repomap",
            "description": "Generate a PageRank-ranked repository map showing the most important symbols and their relationships. Token-budgeted output.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Root directory to scan"},
                    "budget": {
                        "type": "integer",
                        "description": format!(
                            "Max output tokens (default: {})",
                            defaults::DEFAULT_REPOMAP_BUDGET
                        ),
                        "default": defaults::DEFAULT_REPOMAP_BUDGET
                    },
                    "focus": {"type": "string", "description": "Comma-separated files to prioritize in PageRank"}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "awl_hashline",
            "description": "Display file contents with content-hashed line tags (LINE:HASH|content) for stable edit references. Use this before making edits to get deterministic line identifiers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File to read with hashline tags"},
                    "action": {"type": "string", "enum": ["read", "apply"], "description": "read=display with tags, apply=apply edit operations from edits field", "default": "read"},
                    "edits": {"type": "string", "description": "Edit operations to apply (only when action=apply). Format: REPLACE LINE:HASH newcontent / DELETE LINE:HASH / INSERT AFTER LINE:HASH newcontent"}
                },
                "required": ["path"]
            }
        }),
    ];

    if defaults::mcp_agent_enabled() {
        tools.push(json!({
            "name": "awl_agent",
            "description": "Run the full agent loop (Formulate -> Plan -> Execute -> Verify) on a task using a local Ollama model. Long-running operation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task": {"type": "string", "description": "Task description for the agent"},
                    "model": {
                        "type": "string",
                        "description": format!(
                            "Ollama model name (default: {})",
                            agent_model
                        ),
                        "default": agent_model
                    },
                    "mcp_config": {"type": "string", "description": "Path to MCP server config JSON for the agent to use"}
                },
                "required": ["task"]
            }
        }));
    }

    tools
}

fn handle_request(method: &str, params: &Value) -> Result<Option<Value>, JsonRpcError> {
    match method {
        "initialize" => Ok(Some(json!({
            "protocolVersion": negotiated_protocol_version(params),
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {
                "name": "awl",
                "title": "Awl",
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": "Use Awl as a bounded local execution worker. Keep orchestration, architectural judgment, and final review in the host agent; use awl_dispatch level 2 for implementation packets and level 3 for verification packets. awl_agent is hidden unless AWL_ENABLE_MCP_AGENT=1 is set."
        }))),
        "notifications/initialized" => Ok(None),
        "ping" => Ok(Some(json!({}))),
        "tools/list" => Ok(Some(json!({
            "tools": server_tool_definitions()
        }))),
        "tools/call" => Ok(Some(handle_tool_call(params))),
        _ => Err(JsonRpcError::method_not_found()),
    }
}

fn negotiated_protocol_version(params: &Value) -> &'static str {
    let Some(requested) = params.get("protocolVersion").and_then(Value::as_str) else {
        return LATEST_PROTOCOL_VERSION;
    };

    SUPPORTED_PROTOCOL_VERSIONS
        .iter()
        .copied()
        .find(|version| *version == requested)
        .unwrap_or(LATEST_PROTOCOL_VERSION)
}

fn handle_tool_call(params: &Value) -> Value {
    match execute_tool_call(params) {
        Ok(output) => json!({
            "content": [{"type": "text", "text": output}]
        }),
        Err(error) => json!({
            "isError": true,
            "content": [{"type": "text", "text": format!("ERROR: {error}")}]
        }),
    }
}

fn execute_tool_call(params: &Value) -> Result<String, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "tools/call requires string field: name".to_string())?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match name {
        "awl_health" => Ok(execute_health()),
        "awl_dispatch" => execute_dispatch(&args),
        "awl_repomap" => execute_repomap(&args),
        "awl_hashline" => execute_hashline(&args),
        "awl_agent" if defaults::mcp_agent_enabled() => execute_agent(&args),
        "awl_agent" => Err(
            "awl_agent is disabled for MCP by default. Set AWL_ENABLE_MCP_AGENT=1 to expose it."
                .to_string(),
        ),
        other => Err(format!("unknown tool: {other}")),
    }
}

fn execute_health() -> String {
    let cwd = std::env::current_dir().map_or_else(
        |_| "<unknown>".to_string(),
        |path| path.display().to_string(),
    );
    json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "cwd": cwd,
        "tools": health_tool_names(),
        "ollama": "not checked"
    })
    .to_string()
}

fn health_tool_names() -> Vec<&'static str> {
    let mut tools = vec!["awl_health", "awl_dispatch", "awl_repomap", "awl_hashline"];
    if defaults::mcp_agent_enabled() {
        tools.push("awl_agent");
    }
    tools
}

fn execute_dispatch(args: &Value) -> Result<String, String> {
    let level = parse_level(args)?;
    let task = required_string(args, "task")?;
    let context = optional_string(args, "context").unwrap_or_default();
    let constraints = optional_string_array(args, "constraints")?;
    let target_path = optional_string(args, "target_path");
    let target_files = optional_string_array(args, "target_files")?;
    let context_paths = optional_string_array(args, "context_paths")?;
    let verify_command = optional_string(args, "verify_command");
    let apply = args.get("apply").and_then(Value::as_bool).unwrap_or(false);
    let max_attempts = optional_usize(args, "max_attempts")?;
    let max_return_chars = optional_usize(args, "max_return_chars")?;
    let auto_repomap = args
        .get("auto_repomap")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let repomap_focus = optional_string_array(args, "repomap_focus")?;
    let repomap_budget = optional_usize(args, "repomap_budget")?;
    let input = json!({
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
    let mut options = crate::dispatch::DispatchOptions::new(level);
    options.apply = apply;
    options.verify_command = verify_command;
    options.target_path = target_path;
    options.max_attempts = max_attempts;
    options.max_return_chars = max_return_chars;
    options.auto_repomap = auto_repomap;
    options.repomap_focus = repomap_focus;
    options.repomap_budget = repomap_budget;
    crate::dispatch::run_capture(&options, &input.to_string()).map_err(|e| e.to_string())
}

fn execute_repomap(args: &Value) -> Result<String, String> {
    let path = required_string(args, "path")?;
    let budget = match args.get("budget").and_then(Value::as_u64) {
        Some(value) => usize::try_from(value).map_err(|_| "budget is too large".to_string())?,
        None => defaults::DEFAULT_REPOMAP_BUDGET,
    };
    let focus = optional_string(args, "focus")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(PathBuf::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    crate::repomap::generate(Path::new(&path), budget, &focus).map_err(|e| e.to_string())
}

fn execute_hashline(args: &Value) -> Result<String, String> {
    let path = required_string(args, "path")?;
    let action = optional_string(args, "action").unwrap_or_else(|| "read".to_string());
    match action.as_str() {
        "read" => crate::hashline::run_capture(&["read", &path]).map_err(|e| e.to_string()),
        "apply" => {
            let edits = required_string(args, "edits")?;
            crate::hashline::apply_from_string(&path, &edits).map_err(|e| e.to_string())
        }
        other => Err(format!("unsupported hashline action: {other}")),
    }
}

fn execute_agent(args: &Value) -> Result<String, String> {
    let task = required_string(args, "task")?;
    let model = optional_string(args, "model").unwrap_or_else(defaults::configured_agent_model);
    let mcp_config_path = optional_string(args, "mcp_config")
        .map(PathBuf::from)
        .or_else(defaults::configured_mcp_config_path);
    let config = AgentConfig {
        model,
        mcp_config_path,
        ..Default::default()
    };
    let session = Session::new().map_err(|e| e.to_string())?;
    let mut phase_state = PhaseState::new(&task);
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime
        .block_on(async {
            agent::run_agent(&config, &mut phase_state, &session, &task, None).await
        })
        .map_err(|e| e.to_string())
}

fn required_string(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("missing required string field: {key}"))
}

fn optional_string(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
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

fn parse_level(args: &Value) -> Result<u8, String> {
    let level = args
        .get("level")
        .and_then(Value::as_u64)
        .ok_or_else(|| "missing required integer field: level".to_string())?;
    match level {
        2 | 3 => u8::try_from(level).map_err(|_| "level is out of range".to_string()),
        _ => Err("level must be 2 or 3".to_string()),
    }
}

fn jsonrpc_error_response(id: &Value, error: &JsonRpcError) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": error.code,
            "message": error.message
        }
    })
}

fn parse_request(trimmed: &str) -> Result<Value, JsonRpcError> {
    serde_json::from_str(trimmed)
        .map_err(|e| JsonRpcError::invalid_params(format!("invalid JSON-RPC request: {e}")))
}

pub fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request = match parse_request(trimmed) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("[mcp-server] rejected malformed JSON-RPC request");
                let response = jsonrpc_error_response(&Value::Null, &error);
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let Some(id) = request.get("id").cloned() else {
            if method == "notifications/initialized" {
                eprintln!("[mcp-server] initialized");
            } else {
                eprintln!("[mcp-server] notification: {method}");
            }
            continue;
        };

        let response = match handle_request(method, &params) {
            Ok(Some(result)) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }),
            Ok(None) => continue,
            Err(error) => jsonrpc_error_response(&id, &error),
        };

        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_response() {
        let response = handle_request("initialize", &json!({}))
            .expect("initialize should succeed")
            .expect("initialize should return a response");
        assert_eq!(
            response.get("protocolVersion").and_then(Value::as_str),
            Some(LATEST_PROTOCOL_VERSION)
        );
        assert!(response
            .get("capabilities")
            .and_then(|caps| caps.get("tools"))
            .is_some());
        assert_eq!(
            response
                .get("serverInfo")
                .and_then(|info| info.get("name"))
                .and_then(Value::as_str),
            Some("awl")
        );
    }

    #[test]
    fn test_initialize_negotiates_older_supported_protocol() {
        let response = handle_request(
            "initialize",
            &json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0"}
            }),
        )
        .expect("initialize should succeed")
        .expect("initialize should return a response");
        assert_eq!(
            response.get("protocolVersion").and_then(Value::as_str),
            Some("2024-11-05")
        );
    }

    #[test]
    fn test_ping_response() {
        let response = handle_request("ping", &json!({}))
            .expect("ping should succeed")
            .expect("ping should return a response");
        assert_eq!(response, json!({}));
    }

    #[test]
    fn test_tools_list() {
        let response = handle_request("tools/list", &json!({}))
            .expect("tools/list should succeed")
            .expect("tools/list should return a response");
        let tools = response
            .get("tools")
            .and_then(Value::as_array)
            .expect("tools/list response should contain tools");
        let names = tools
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["awl_health", "awl_dispatch", "awl_repomap", "awl_hashline"]
        );
    }

    #[test]
    fn test_health_tool() {
        let response = handle_tool_call(&json!({"name": "awl_health"}));
        let text = response
            .get("content")
            .and_then(Value::as_array)
            .and_then(|content| content.first())
            .and_then(|item| item.get("text"))
            .and_then(Value::as_str)
            .expect("health response should contain text");
        let health: Value = serde_json::from_str(text).expect("health response should be JSON");
        assert_eq!(health.get("status").and_then(Value::as_str), Some("ok"));
        assert_eq!(
            health.get("version").and_then(Value::as_str),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(
            health
                .get("tools")
                .and_then(Value::as_array)
                .expect("health response should list tools")
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>(),
            vec!["awl_health", "awl_dispatch", "awl_repomap", "awl_hashline"]
        );
    }

    #[test]
    fn test_agent_tool_is_disabled_by_default() {
        let response = handle_tool_call(&json!({
            "name": "awl_agent",
            "arguments": {"task": "do something"}
        }));
        assert_eq!(response.get("isError").and_then(Value::as_bool), Some(true));
        let text = response
            .get("content")
            .and_then(Value::as_array)
            .and_then(|content| content.first())
            .and_then(|item| item.get("text"))
            .and_then(Value::as_str)
            .expect("error response should contain text");
        assert!(text.contains("AWL_ENABLE_MCP_AGENT"));
    }

    #[test]
    fn test_unknown_method() {
        let error = handle_request("bogus", &json!({})).expect_err("bogus should fail");
        assert_eq!(error.code, -32601);
    }

    #[test]
    fn test_dispatch_tool_schema() {
        let tools = server_tool_definitions();
        let dispatch = tools
            .into_iter()
            .find(|tool| tool.get("name").and_then(Value::as_str) == Some("awl_dispatch"))
            .expect("dispatch tool should exist");
        let schema = dispatch
            .get("inputSchema")
            .expect("dispatch tool should have a schema");
        assert_eq!(
            schema
                .get("properties")
                .and_then(|props| props.get("level"))
                .and_then(|level| level.get("type"))
                .and_then(Value::as_str),
            Some("integer")
        );
        assert_eq!(
            schema
                .get("properties")
                .and_then(|props| props.get("task"))
                .and_then(|task| task.get("type"))
                .and_then(Value::as_str),
            Some("string")
        );
        assert_eq!(
            schema
                .get("required")
                .and_then(Value::as_array)
                .expect("dispatch required should exist")
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>(),
            vec!["level", "task"]
        );
        assert_eq!(
            schema
                .get("properties")
                .and_then(|props| props.get("level"))
                .and_then(|level| level.get("enum"))
                .and_then(Value::as_array)
                .expect("dispatch level enum should exist")
                .iter()
                .filter_map(Value::as_u64)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
    }
}
