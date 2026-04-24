use std::fmt::Write as _;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::defaults;
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
            model: defaults::configured_agent_model(),
            base_url: defaults::configured_ollama_base_url(),
            max_tokens: 4096,
            max_iterations: 30,
            temperature: 0.2,
            mcp_config_path: defaults::configured_mcp_config_path(),
        }
    }
}

const COMPACTION_THRESHOLD: usize = 3000;
const KEEP_RECENT: usize = 6;

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
    let url = defaults::ollama_chat_completions_url(&config.base_url);

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
    refresh_system_message(&mut messages, phase_state);
    let mut consecutive_text_count: u32 = 0;

    for _iteration in 0..config.max_iterations {
        if estimate_tokens(&messages) > COMPACTION_THRESHOLD {
            if let Err(error) =
                compact_messages(&mut messages, phase_state, config, &client, &url).await
            {
                eprintln!("[compact] failed: {error}");
            }
        }

        let request = json!({
            "model": config.model,
            "messages": messages,
            "tools": registry.definitions(),
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "keep_alive": -1,
            "stream": false
        });

        let response = client.post(&url).json(&request).send().await.map_err(|e| {
            format!(
                "Ollama unreachable at {}: {e}. Is `ollama serve` running?",
                config.base_url
            )
        })?;
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
        let content = choice.get("content").and_then(Value::as_str).unwrap_or("");
        if capture_evidence(content, phase_state) {
            session.update_metadata(phase_state)?;
            refresh_system_message(&mut messages, phase_state);
        }
        session.append(&assistant_msg)?;
        messages.push(assistant_msg);
        if phase_state.current != Phase::Complete {
            emit_phase_output(phase_state.current, content);
        }

        let tool_calls = choice
            .get("tool_calls")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        if tool_calls.is_empty() {
            // Fallback: try to parse an inline tool call from content.
            if let Some((name, args)) = parse_inline_tool_call(content) {
                if let Some(path) = extract_tool_path(&args) {
                    eprintln!("[tool-inline] {name} -> {path}");
                } else {
                    eprintln!("[tool-inline] {name}");
                }
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
                consecutive_text_count = 0;
                continue;
            }

            // Check for phase gate signal.
            match phases::detect_gate(phase_state.current, content) {
                Some(GateSignal::Advance) => {
                    consecutive_text_count = 0;
                    eprintln!("[agent] phase {} complete", phase_state.current.name());
                    if phase_state.advance().is_none() {
                        session.update_metadata(phase_state)?;
                        return Ok(content.to_string());
                    }
                    session.update_metadata(phase_state)?;
                    refresh_system_message(&mut messages, phase_state);
                    let injected = json!({
                        "role":"user",
                        "content": phases::phase_system_prompt(phase_state.current)
                    });
                    session.append(&injected)?;
                    messages.push(injected);
                }
                Some(GateSignal::Regress) => {
                    consecutive_text_count = 0;
                    eprintln!("[agent] verification failed; regressing to execute");
                    phase_state.regress_to_execute()?;
                    session.update_metadata(phase_state)?;
                    refresh_system_message(&mut messages, phase_state);
                    let prompt = phases::phase_system_prompt(Phase::Execute);
                    let injected = json!({
                        "role":"user",
                        "content": format!(
                            "Verification failed with issues:\n{}\n\n{prompt}",
                            truncate(content, 2000)
                        )
                    });
                    session.append(&injected)?;
                    messages.push(injected);
                }
                None => {
                    if phase_state.current == Phase::Complete {
                        return Ok(content.to_string());
                    }
                    consecutive_text_count += 1;
                    if consecutive_text_count >= 3 {
                        let prior_phase = phase_state.current;
                        let handoff = format!(
                            "NEEDS_HUMAN_REVIEW: Agent produced {} consecutive text responses \
without tool use or phase completion in {} phase. Last output: {}\nSession: {}",
                            consecutive_text_count,
                            prior_phase.name(),
                            truncate(content, 500),
                            session.id()
                        );
                        eprintln!("[agent] {handoff}");
                        phase_state
                            .phase_notes
                            .insert("prior_phase".to_string(), prior_phase.name().to_string());
                        phase_state
                            .phase_notes
                            .insert("handoff_reason".to_string(), handoff.clone());
                        phase_state.current = Phase::NeedsHuman;
                        session.update_metadata(phase_state)?;
                        return Err(handoff.into());
                    }
                    let nudge = json!({
                        "role": "user",
                        "content": format!(
                            "You are in the {} phase. Your response did not include a tool call or phase signal. \
                    Either use a tool to make progress, or signal completion with {}.",
                            phase_state.current.name(),
                            phase_completion_signal(phase_state.current)
                        )
                    });
                    session.append(&nudge)?;
                    messages.push(nudge);
                }
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
            let call_id = tc.get("id").and_then(Value::as_str).unwrap_or("");

            let parsed_args: Value = match serde_json::from_str(arguments) {
                Ok(value) => value,
                Err(error) => {
                    consecutive_text_count = 0;
                    let tool_message = json!({
                        "role":"tool",
                        "tool_call_id": call_id,
                        "content": format!("ERROR: invalid tool arguments JSON for {name}: {error}")
                    });
                    session.append(&tool_message)?;
                    messages.push(tool_message);
                    continue;
                }
            };

            if let Some(path) = extract_tool_path(&parsed_args) {
                eprintln!("[tool] {name} -> {path}");
            } else {
                eprintln!("[tool] {name}");
            }
            consecutive_text_count = 0;
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

    let prior_phase = phase_state.current;
    phase_state
        .phase_notes
        .insert("prior_phase".to_string(), prior_phase.name().to_string());
    phase_state.phase_notes.insert(
        "handoff_reason".to_string(),
        format!(
            "Exceeded max iterations ({}) during {} phase",
            config.max_iterations,
            prior_phase.name()
        ),
    );
    phase_state.current = Phase::NeedsHuman;
    session.update_metadata(phase_state)?;
    eprintln!(
        "[agent] NEEDS_HUMAN: exceeded max iterations in {} phase. Resume with: awl agent --resume {}",
        prior_phase.name(),
        session.id()
    );
    Ok(format!(
        "NEEDS_HUMAN_REVIEW: Agent reached iteration limit during {} phase. Session {} saved. Resume with --resume {}.",
        prior_phase.name(),
        session.id(),
        session.id()
    ))
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
    let mut prompt = String::new();

    if let Some(persona) = &phase_state.persona {
        let _ = write!(prompt, "You are {persona}.\n\n");
    } else {
        prompt.push_str("You are an autonomous coding agent.\n\n");
    }

    if let Some(goal) = &phase_state.goal {
        let _ = write!(prompt, "Research goal: {goal}\n\n");
    }

    let _ = write!(prompt, "Task: {}\n\n", phase_state.task_description);

    if !phase_state.ideas.is_empty() {
        prompt.push_str("User-supplied ideas and directions:\n");
        for idea in &phase_state.ideas {
            let _ = writeln!(prompt, "- {idea}");
        }
        prompt.push('\n');
    }

    if !phase_state.evidence.is_empty() {
        prompt.push_str("Accumulated evidence/findings:\n");
        for finding in &phase_state.evidence {
            let _ = writeln!(prompt, "- {finding}");
        }
        prompt.push('\n');
    }

    prompt.push_str(phases::phase_system_prompt(phase_state.current));
    prompt.push_str(
        "\n\nUse available tools when needed. Keep changes deterministic. \
When you discover a key finding, include it in your response prefixed with EVIDENCE: \
so it can be preserved across session compaction.",
    );

    prompt
}

#[allow(clippy::too_many_lines)]
pub fn run_agent_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut task = String::new();
    let mut resume_id: Option<String> = None;
    let mut model = defaults::configured_agent_model();
    let mut mcp_config: Option<PathBuf> = None;
    let mut persona: Option<String> = None;
    let mut goal: Option<String> = None;
    let mut ideas: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--task" => {
                i += 1;
                task = args.get(i).cloned().ok_or("--task requires a value")?;
            }
            "--resume" => {
                i += 1;
                resume_id = Some(args.get(i).cloned().ok_or("--resume requires a value")?);
            }
            "--model" => {
                i += 1;
                model = args.get(i).cloned().ok_or("--model requires a value")?;
            }
            "--mcp-config" => {
                i += 1;
                mcp_config = Some(PathBuf::from(
                    args.get(i).ok_or("--mcp-config requires a value")?,
                ));
            }
            "--persona" => {
                i += 1;
                persona = Some(args.get(i).cloned().ok_or("--persona requires a value")?);
            }
            "--goal" => {
                i += 1;
                goal = Some(args.get(i).cloned().ok_or("--goal requires a value")?);
            }
            "--idea" => {
                i += 1;
                ideas.push(args.get(i).cloned().ok_or("--idea requires a value")?);
            }
            other => return Err(format!("unknown agent flag: {other}").into()),
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
    let config = AgentConfig {
        model,
        mcp_config_path: mcp_config.or_else(defaults::configured_mcp_config_path),
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
            if phase_state.current == Phase::NeedsHuman {
                let prior_phase = phase_state
                    .phase_notes
                    .get("prior_phase")
                    .and_then(|phase| match phase.as_str() {
                        "Formulate" => Some(Phase::Formulate),
                        "Plan" => Some(Phase::Plan),
                        "Execute" => Some(Phase::Execute),
                        "Verify" => Some(Phase::Verify),
                        _ => None,
                    })
                    .unwrap_or(Phase::Execute);
                phase_state.current = prior_phase;
                eprintln!("[agent] resuming from NeedsHuman -> {}", prior_phase.name());
            }
            // If --task was provided on resume, update the task description.
            if !task.is_empty() {
                phase_state.task_description = task.clone();
            }
            if let Some(persona) = persona.clone() {
                phase_state.persona = Some(persona);
            }
            if let Some(goal) = goal.clone() {
                phase_state.goal = Some(goal);
            }
            if !ideas.is_empty() {
                phase_state.ideas = ideas.clone();
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
            phase_state.persona = persona;
            phase_state.goal = goal;
            phase_state.ideas = ideas;
            let result = run_agent(&config, &mut phase_state, &sess, &task, None).await?;
            println!("{result}");
        }
        Ok(())
    })
}

fn phase_completion_signal(phase: Phase) -> &'static str {
    match phase {
        Phase::Formulate => "FORMULATE_COMPLETE",
        Phase::Plan => "PLAN_COMPLETE",
        Phase::Execute => "EXECUTE_COMPLETE",
        Phase::Verify => "VERIFY_COMPLETE",
        Phase::Complete => "TASK_COMPLETE",
        Phase::NeedsHuman => "NEEDS_HUMAN_REVIEW",
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

fn estimate_tokens(messages: &[Value]) -> usize {
    messages
        .iter()
        .map(|message| {
            let content_len = message
                .get("content")
                .and_then(Value::as_str)
                .map_or(0, str::len);
            let tool_calls_len =
                message
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .map_or(0, |calls| {
                        calls
                            .iter()
                            .map(|tc| serde_json::to_string(tc).map_or(50, |s| s.len()))
                            .sum::<usize>()
                    });
            // ~3.5 chars per token on average for mixed code/prose, plus per-message framing.
            (content_len + tool_calls_len) * 2 / 7 + 4
        })
        .sum()
}

async fn compact_messages(
    messages: &mut Vec<Value>,
    phase_state: &PhaseState,
    config: &AgentConfig,
    client: &reqwest::Client,
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if messages.len() <= KEEP_RECENT + 2 {
        return Ok(());
    }

    let system_msg = messages.first().cloned();
    let to_summarize = messages[1..messages.len() - KEEP_RECENT].to_vec();
    let recent = messages[messages.len() - KEEP_RECENT..].to_vec();

    let mut summary_input = String::new();
    for msg in &to_summarize {
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("?");
        let content = msg.get("content").and_then(Value::as_str).unwrap_or("");
        if !content.is_empty() {
            let _ = writeln!(summary_input, "[{role}] {}", truncate(content, 300));
        }
    }

    let summary_prompt = format!(
        "Summarize the following conversation history in <=200 words. Preserve: key decisions, \
file paths modified, tool outputs, error messages, and current progress toward the task. \
Current phase: {}.\n\n{}",
        phase_state.current.name(),
        summary_input
    );

    let request = json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": "You are a conversation summarizer. Output only the summary, no preamble."},
            {"role": "user", "content": summary_prompt}
        ],
        "max_tokens": 512,
        "temperature": 0.1,
        "stream": false
    });

    let response = client.post(url).json(&request).send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("summary request failed with {status}: {body}").into());
    }
    let raw: Value = response.json().await?;
    let summary_text = raw
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|value| value.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("[compaction failed - no summary generated]");

    let compacted_msg = json!({
        "role": "user",
        "content": format!(
            "[COMPACTED CONTEXT - {} messages summarized]\n{}",
            to_summarize.len(),
            summary_text
        )
    });

    let original_len = to_summarize.len() + KEEP_RECENT + usize::from(system_msg.is_some());
    let saved_tokens = estimate_tokens(&to_summarize);

    messages.clear();
    if let Some(system_msg) = system_msg {
        messages.push(system_msg);
    }
    messages.push(compacted_msg);
    messages.extend(recent);

    eprintln!(
        "[compact] compacted: {} messages -> {} (saved ~{} tokens)",
        original_len,
        messages.len(),
        saved_tokens
    );

    Ok(())
}

fn refresh_system_message(messages: &mut Vec<Value>, phase_state: &PhaseState) {
    let system_content = json!(build_system_prompt(phase_state));
    if let Some(system_msg) = messages.first_mut() {
        if system_msg.get("role").and_then(Value::as_str) == Some("system") {
            system_msg["content"] = system_content;
            return;
        }
    }
    messages.insert(0, json!({"role":"system","content":system_content}));
}

const MAX_EVIDENCE: usize = 20;

fn capture_evidence(content: &str, phase_state: &mut PhaseState) -> bool {
    let mut changed = false;
    for line in content.lines() {
        if let Some(evidence) = line.strip_prefix("EVIDENCE:") {
            let finding = evidence.trim().to_string();
            if !finding.is_empty() && !phase_state.evidence.contains(&finding) {
                phase_state.evidence.push(finding);
                changed = true;
            }
        }
    }
    // Keep only the most recent findings to prevent system prompt bloat.
    if phase_state.evidence.len() > MAX_EVIDENCE {
        let drain_count = phase_state.evidence.len() - MAX_EVIDENCE;
        phase_state.evidence.drain(..drain_count);
    }
    changed
}

/// Gate signal keywords to filter from display output.
const GATE_SIGNALS: &[&str] = &[
    "FORMULATE_COMPLETE",
    "PLAN_COMPLETE",
    "EXECUTE_COMPLETE",
    "VERIFY_COMPLETE",
    "VERIFY_FAILED",
];

/// Print the model's substantive output to stderr, prefixed by current phase.
/// Filters out: gate signal lines, empty lines, markdown code fences, and
/// raw JSON blocks (which are inline tool calls, not user-facing content).
fn emit_phase_output(phase: Phase, content: &str) {
    let prefix = format!("[{}]", phase.name().to_lowercase());
    let mut in_json_block = false;
    let mut in_code_fence = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines.
        if trimmed.is_empty() {
            continue;
        }

        // Skip gate signal lines.
        let upper = trimmed.to_ascii_uppercase();
        if GATE_SIGNALS.iter().any(|sig| upper == *sig) {
            continue;
        }

        // Track markdown code fences — skip the fence markers themselves
        // and suppress JSON/raw blocks inside them.
        if trimmed.starts_with("```") {
            if in_code_fence {
                // Closing fence.
                in_code_fence = false;
                in_json_block = false;
            } else {
                // Opening fence — check if it's a JSON block (inline tool call).
                in_code_fence = true;
                in_json_block = trimmed == "```json";
            }
            continue;
        }

        // Skip lines inside a JSON code block (inline tool call bodies).
        if in_json_block {
            continue;
        }

        // Skip bare JSON objects outside fences (unfenced inline tool calls).
        if !in_code_fence && trimmed.starts_with('{') && trimmed.contains("\"name\"") {
            continue;
        }

        eprintln!("{prefix} {trimmed}");
    }
}

/// Extract a file path from tool call arguments for display purposes.
fn extract_tool_path(args: &Value) -> Option<&str> {
    args.get("path")
        .and_then(Value::as_str)
        .or_else(|| args.get("file_path").and_then(Value::as_str))
        .or_else(|| args.get("file").and_then(Value::as_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_completion_signal() {
        assert_eq!(
            phase_completion_signal(Phase::Formulate),
            "FORMULATE_COMPLETE"
        );
        assert_eq!(phase_completion_signal(Phase::Plan), "PLAN_COMPLETE");
        assert_eq!(phase_completion_signal(Phase::Execute), "EXECUTE_COMPLETE");
        assert_eq!(phase_completion_signal(Phase::Verify), "VERIFY_COMPLETE");
        assert_eq!(phase_completion_signal(Phase::Complete), "TASK_COMPLETE");
        assert_eq!(
            phase_completion_signal(Phase::NeedsHuman),
            "NEEDS_HUMAN_REVIEW"
        );
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 5), "hello");
        assert_eq!(truncate("hello world", 5), "hello");
        assert_eq!(truncate("hi", 10), "hi");
    }

    #[test]
    fn test_refresh_system_message_updates_existing_prompt() {
        let mut phase_state = PhaseState::new("test task");
        phase_state
            .evidence
            .push("found a relevant file".to_string());
        let mut messages = vec![json!({"role":"system","content":"stale"})];

        refresh_system_message(&mut messages, &phase_state);

        let content = messages[0]["content"].as_str().unwrap_or("");
        assert!(content.contains("found a relevant file"));
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_extract_tool_path_variants() {
        let with_path = json!({"path": "src/main.rs"});
        assert_eq!(extract_tool_path(&with_path), Some("src/main.rs"));

        let with_file_path = json!({"file_path": "Cargo.toml"});
        assert_eq!(extract_tool_path(&with_file_path), Some("Cargo.toml"));

        let with_file = json!({"file": "test.py"});
        assert_eq!(extract_tool_path(&with_file), Some("test.py"));

        let without = json!({"command": "cargo test"});
        assert_eq!(extract_tool_path(&without), None);
    }

    #[test]
    fn test_gate_signals_filtered() {
        // Verify the GATE_SIGNALS list matches phase_completion_signal outputs.
        assert!(GATE_SIGNALS.contains(&"FORMULATE_COMPLETE"));
        assert!(GATE_SIGNALS.contains(&"PLAN_COMPLETE"));
        assert!(GATE_SIGNALS.contains(&"EXECUTE_COMPLETE"));
        assert!(GATE_SIGNALS.contains(&"VERIFY_COMPLETE"));
        assert!(GATE_SIGNALS.contains(&"VERIFY_FAILED"));
    }

    #[test]
    fn test_refresh_system_message_inserts_missing_system_prompt() {
        let phase_state = PhaseState::new("test task");
        let mut messages = vec![json!({"role":"user","content":"hello"})];

        refresh_system_message(&mut messages, &phase_state);

        assert_eq!(messages[0]["role"].as_str(), Some("system"));
        assert_eq!(messages[1]["role"].as_str(), Some("user"));
    }
}
