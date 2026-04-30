use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

const MCP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

#[derive(Debug, Clone, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct McpConfigFile {
    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, McpServerEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct McpServerEntry {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct McpToolSpec {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

struct McpInner {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

pub struct McpClient {
    pub server_name: String,
    inner: Mutex<McpInner>,
}

impl McpClient {
    pub async fn connect(config: &McpServerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());
        cmd.envs(&config.env);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn MCP server {}: {e}", config.name))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| format!("MCP server {} missing stdin pipe", config.name))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| format!("MCP server {} missing stdout pipe", config.name))?;
        let stdout = BufReader::new(stdout);

        let client = Self {
            server_name: config.name.clone(),
            inner: Mutex::new(McpInner {
                _child: child,
                stdin,
                stdout,
                next_id: 1,
            }),
        };

        let init = json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "awl", "version": env!("CARGO_PKG_VERSION")}
        });
        let _ = client.request("initialize", init).await?;
        client
            .notify("notifications/initialized", json!({}))
            .await?;
        Ok(client)
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolSpec>, Box<dyn std::error::Error>> {
        let resp = self.request("tools/list", json!({})).await?;
        let tools = resp
            .get("tools")
            .and_then(Value::as_array)
            .ok_or("tools/list response missing tools array")?;
        let mut specs = Vec::with_capacity(tools.len());
        for tool in tools {
            let name = tool
                .get("name")
                .and_then(Value::as_str)
                .ok_or("tool missing name")?
                .to_string();
            let description = tool
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let schema = tool
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| json!({"type":"object","properties":{}}));
            specs.push(McpToolSpec {
                name,
                description,
                schema,
            });
        }
        Ok(specs)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        args: Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let resp = self
            .request(
                "tools/call",
                json!({
                    "name": name,
                    "arguments": args
                }),
            )
            .await?;

        let mut out = String::new();
        if let Some(content) = resp.get("content").and_then(Value::as_array) {
            for item in content {
                if let Some(text) = item.get("text").and_then(Value::as_str) {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(text);
                } else if !item.is_null() {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(&item.to_string());
                }
            }
        }
        if out.is_empty() {
            out = resp.to_string();
        }
        if resp.get("isError").and_then(Value::as_bool) == Some(true) {
            Err(format!("mcp tool error from {}: {out}", self.server_name).into())
        } else {
            Ok(out)
        }
    }

    async fn notify(&self, method: &str, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        let encoded = serde_json::to_string(&msg)?;
        let mut inner = self.inner.lock().await;
        inner.stdin.write_all(encoded.as_bytes()).await?;
        inner.stdin.write_all(b"\n").await?;
        inner.stdin.flush().await?;
        Ok(())
    }

    async fn request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let mut inner = self.inner.lock().await;
        let id = inner.next_id;
        inner.next_id = inner
            .next_id
            .checked_add(1)
            .ok_or("mcp request id overflow")?;

        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let encoded = serde_json::to_string(&msg)?;
        inner.stdin.write_all(encoded.as_bytes()).await?;
        inner.stdin.write_all(b"\n").await?;
        inner.stdin.flush().await?;

        let mut line = String::new();
        loop {
            line.clear();
            let read = timeout(MCP_REQUEST_TIMEOUT, inner.stdout.read_line(&mut line))
                .await
                .map_err(|_| {
                    format!(
                        "timed out waiting for mcp response from {}",
                        self.server_name
                    )
                })??;
            if read == 0 {
                return Err("mcp server closed stdout".into());
            }
            let value: Value = serde_json::from_str(line.trim())?;
            let incoming_id = value.get("id").and_then(Value::as_u64);
            if incoming_id != Some(id) {
                continue;
            }
            if let Some(err) = value.get("error") {
                return Err(format!("mcp error from {}: {err}", self.server_name).into());
            }
            let result = value
                .get("result")
                .cloned()
                .ok_or("mcp response missing result")?;
            return Ok(result);
        }
    }
}

pub fn load_mcp_config(path: &Path) -> Result<Vec<McpServerConfig>, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    let parsed: McpConfigFile = serde_json::from_str(&raw)?;
    let mut out = Vec::with_capacity(parsed.mcp_servers.len());
    for (name, cfg) in parsed.mcp_servers {
        out.push(McpServerConfig {
            name,
            command: cfg.command,
            args: cfg.args,
            env: cfg.env,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub type SharedMcpClient = Arc<McpClient>;
