use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::mcp_client::{load_mcp_config, McpClient};
use crate::phases::{self, GateSignal, Phase, PhaseState};
use crate::session::Session;
use crate::tools::{McpToolProxy, ToolRegistry};

pub struct AgentConfig {
    pub model: String,
    pub base_url: String,
    pub max_tokens: u32,
    pub max_iterations: usize,
    pub temperature: f64,
    pub mcp_config_path: Option<PathBuf>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "qwen2.5-coder:14b".to_string(),
            base_url: "http://127.0.0.1:11434/v1".to_string(),
            max_tokens: 4096,
            max_iterations: 30,
            temperature: 0.2,
            mcp_config_path: None,
        }
    }
}

#[allow(clippy::too_many_lines)]
pub async fn run_agent(
    config: &AgentConfig,
    phase_state: &mut PhaseState,
    session: &Session,
    initial_task: &str,
    resumed_messages: Option<Vec<Value>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut registry = ToolRegistry::new();
    if let Some(path) = &config.mcp_config_path {
        register_mcp_tools(&mut registry, path).await?;
    }

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let mut messages: Vec<Value> = if let Some(msgs) = resumed_messages {
        eprintln!(
            "[agent] resumed with {} messages, phase: {}",
            msgs.len(),
            phase_state.current.name()
        );
        msgs
    } else {
        // New session: write metadata and initial messages.
        session.write_metadata(phase_state)?;
        let system_msg = json!({"role":"system","content":build_system_prompt(phase_state)});
        let user_msg = json!({"role":"user","content":initial_task});
        session.append(&system_msg)?;
        session.append(&user_msg)?;
        vec![system_msg, user_msg]
    };

    for _iteration in 0..config.max_iterations {
        let request = json!({
            "model": config.model,
            "messages": messages,
            "tools": registry.definitions(),
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "stream": false
        });

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Ollama unreachable: {e}"))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Ollama returned {status}: {body}").into());
        }

        let raw: Value = response.json().await?;
        let choice = raw
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("message"))
            .cloned()
            .ok_or("missing choices[0].message in model response")?;

        let assistant_msg = json!({
            "role": "assistant",
            "content": choice.get("content").and_then(Value::as_str).unwrap_or(""),
            "tool_calls": choice.get("tool_calls").cloned().unwrap_or_else(|| json!([]))
        });
        session.append(&assistant_msg)?;
        messages.push(assistant_msg);

        let tool_calls = choice
            .get("tool_calls")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        if tool_calls.is_empty() {
            let content = choice.get("content").and_then(Value::as_str).unwrap_or("");

            // Fallback: try to parse an inline tool call from content.
            if let Some((name, args)) = parse_inline_tool_call(content) {
                eprintln!("[tool-inline] {name}");
                let tool_output = match registry.execute(&name, args).await {
                    Ok(output) => output,
                    Err(error) => format!("ERROR: {error}"),
                };
                let tool_message = json!({
                    "role":"tool",
                    "tool_call_id":"inline-0",
                    "content": tool_output
                });
                session.append(&tool_message)?;
                messages.push(tool_message);
                continue;
            }

            // Check for phase gate signal.
            match phases::detect_gate(content) {
                Some(GateSignal::Advance) => {
                    eprintln!("[agent] phase {} complete", phase_state.current.name());
                    if phase_state.advance().is_none() {
                        session.update_metadata(phase_state)?;
                        return Ok(content.to_string());
                    }
                    session.update_metadata(phase_state)?;
                    let injected = json!({
                        "role":"user",
                        "content": phases::phase_system_prompt(phase_state.current)
                    });
                    session.append(&injected)?;
                    messages.push(injected);
                }
                Some(GateSignal::Regress) => {
                    eprintln!("[agent] verification failed; regressing to execute");
                    phase_state.regress_to_execute()?;
                    session.update_metadata(phase_state)?;
                    let prompt = phases::phase_system_prompt(Phase::Execute);
                    let injected = json!({
                        "role":"user",
                        "content": format!("Verification failed with issues:\n{content}\n\n{prompt}")
                    });
                    session.append(&injected)?;
                    messages.push(injected);
                }
                None => return Ok(content.to_string()),
            }
            continue;
        }

        // Execute each tool call.
        for tc in tool_calls {
            let name = tc
                .get("function")
                .and_then(|v| v.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let arguments = tc
                .get("function")
                .and_then(|v| v.get("arguments"))
                .and_then(Value::as_str)
                .unwrap_or("{}");
            let parsed_args: Value = serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));
            let call_id = tc.get("id").and_then(Value::as_str).unwrap_or("");

            eprintln!("[tool] {name}");
            let tool_output = match registry.execute(name, parsed_args).await {
                Ok(output) => output,
                Err(error) => format!("ERROR: {error}"),
            };

            let tool_message = json!({
                "role":"tool",
                "tool_call_id": call_id,
                "content": tool_output
            });
            session.append(&tool_message)?;
            messages.push(tool_message);
        }
    }

    Err(format!("agent exceeded max iterations ({})", config.max_iterations).into())
}

fn parse_inline_tool_call(content: &str) -> Option<(String, Value)> {
    for (idx, ch) in content.char_indices() {
        if ch != '{' {
            continue;
        }
        let slice = &content[idx..];
        let mut stream = serde_json::Deserializer::from_str(slice).into_iter::<Value>();
        let Ok(parsed) = stream.next()? else {
            continue;
        };
        let name = parsed.get("name").and_then(Value::as_str)?;
        let arguments = parsed
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        return Some((name.to_string(), arguments));
    }
    None
}

async fn register_mcp_tools(
    registry: &mut ToolRegistry,
    mcp_config_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let configs = load_mcp_config(mcp_config_path)?;
    for config in configs {
        let client = Arc::new(McpClient::connect(&config).await?);
        let tools = client.list_tools().await?;
        for tool in tools {
            let name = format!("{}::{}", config.name, tool.name);
            let proxy = McpToolProxy::new(name, tool.description, tool.schema, client.clone());
            registry.register(Arc::new(proxy));
        }
    }
    Ok(())
}

fn build_system_prompt(phase_state: &PhaseState) -> String {
    format!(
        "You are an autonomous coding agent.
Task: {}

{}

Use available tools when needed and keep changes deterministic.",
        phase_state.task_description,
        phases::phase_system_prompt(phase_state.current)
    )
}

pub fn run_agent_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut task = String::new();
    let mut offline = false;
    let mut resume_id: Option<String> = None;
    let mut model = "qwen2.5-coder:14b".to_string();
    let mut mcp_config: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--task" => {
                i += 1;
                task = args.get(i).cloned().unwrap_or_default();
            }
            "--offline" => offline = true,
            "--resume" => {
                i += 1;
                resume_id = args.get(i).cloned();
            }
            "--model" => {
                i += 1;
                model = args.get(i).cloned().unwrap_or(model);
            }
            "--mcp-config" => {
                i += 1;
                mcp_config = args.get(i).map(PathBuf::from);
            }
            _ => {}
        }
        i += 1;
    }

    if task.is_empty() && resume_id.is_none() {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        task = buf.trim().to_string();
    }

    if task.is_empty() && resume_id.is_none() {
        return Err("--task <description> or --resume <session-id> required".into());
    }
    if offline {
        eprintln!("[agent] offline mode enabled; keep network restricted to localhost");
        run_offline_hooks();
    }

    let config = AgentConfig {
        model,
        mcp_config_path: mcp_config,
        ..Default::default()
    };

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async move {
        if let Some(id) = resume_id {
            // Resume: load phase state and messages from the session log.
            let resumed = crate::session::Session::resume(&id)?;
            let mut phase_state = resumed.phase_state.unwrap_or_else(|| {
                eprintln!("[agent] no saved phase state found; starting from Formulate");
                PhaseState::new(&task)
            });
            // If --task was provided on resume, update the task description.
            if !task.is_empty() {
                phase_state.task_description = task.clone();
            }
            let effective_task = phase_state.task_description.clone();
            eprintln!(
                "[agent] resuming session {}, phase: {}, {} prior messages",
                id,
                phase_state.current.name(),
                resumed.messages.len()
            );
            let result = run_agent(
                &config,
                &mut phase_state,
                &resumed.session,
                &effective_task,
                Some(resumed.messages),
            )
            .await?;
            println!("{result}");
        } else {
            // New session.
            let sess = Session::new()?;
            eprintln!("[agent] session: {}", sess.id());
            let mut phase_state = PhaseState::new(&task);
            let result = run_agent(&config, &mut phase_state, &sess, &task, None).await?;
            println!("{result}");
        }
        Ok(())
    })
}

fn run_offline_hooks() {
    let Ok(home) = std::env::var("HOME") else {
        return;
    };
    let hooks_dir = PathBuf::from(&home).join("claw/hooks");
    if !hooks_dir.exists() {
        return;
    }

    let snapshot = hooks_dir.join("snapshot.sh");
    if snapshot.exists() {
        eprintln!("[hooks] running snapshot.sh");
        let _ = std::process::Command::new("bash").arg(&snapshot).status();
    }

    let pre_agent = hooks_dir.join("pre-agent.sh");
    if pre_agent.exists() {
        eprintln!("[hooks] running pre-agent.sh");
        let _ = std::process::Command::new("bash")
            .arg(&pre_agent)
            .arg(".")
            .status();
    }
}
