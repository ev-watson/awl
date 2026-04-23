#![allow(clippy::format_push_string, clippy::map_unwrap_or)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::defaults;
use crate::llm_io::{sanitize_json_strings, strip_code_fences};

#[derive(Debug, Deserialize)]
struct PlanSpec {
    task: String,
    #[serde(default)]
    context: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f64,
    stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

const PLAN_SYSTEM_PROMPT: &str = "\
You are a software architect. Given a task and optional codebase context, produce a \
structured implementation plan as a JSON object.

Respond with ONLY a JSON object containing:
- \"status\": \"ok\" or \"error\"
- \"plan\": an array of steps, each with:
    - \"step\": integer step number (1-based)
    - \"task\": string description of what to do
    - \"level\": 2 (implementation) or 3 (verification/lint)
    - \"files\": array of file paths this step will touch (empty if unknown)
    - \"depends_on\": array of step numbers this step depends on (empty if none)
- \"explanation\": brief rationale for the plan structure

Order steps by dependency. Keep plans to 3-8 steps. \
Respond ONLY with valid JSON. No markdown fences, no text outside the JSON.";

pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Parse --level flag (default 2).
    let level: u8 = args
        .iter()
        .position(|a| a == "--level")
        .and_then(|pos| args.get(pos + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);

    if level != 2 && level != 3 {
        return Err("plan --level must be 2 or 3".into());
    }

    let model = defaults::model_for_level(level)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;

    let spec: PlanSpec = serde_json::from_str(&input)
        .or_else(|_| {
            let sanitized = sanitize_json_strings(&input);
            serde_json::from_str(&sanitized)
        })
        .map_err(|e| format!("invalid JSON on stdin: {e}"))?;

    let mut user_msg = format!("Task: {}", spec.task);
    if !spec.context.is_empty() {
        user_msg.push_str(&format!("\n\nCodebase context:\n{}", spec.context));
    }

    let base_url = defaults::configured_ollama_base_url();
    let url = defaults::ollama_chat_completions_url(&base_url);

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: PLAN_SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_msg,
            },
        ],
        max_tokens: 4096,
        temperature: 0.1,
        stream: false,
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("failed to reach Ollama at {url}: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Ollama returned {status}: {body}").into());
        }

        let chat: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("failed to parse response: {e}"))?;

        let raw = chat
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .unwrap_or("");
        let text = strip_code_fences(raw);

        let output: Value = serde_json::from_str(&text).unwrap_or_else(|_| {
            serde_json::json!({
                "status": "error",
                "plan": [],
                "explanation": format!("model did not return valid JSON. Raw output: {text}")
            })
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    })
}
