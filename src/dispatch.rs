#![allow(clippy::doc_markdown, clippy::format_push_string)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::defaults;
use crate::llm_io::{sanitize_json_strings, strip_code_fences};

const MAX_RETRIES: usize = 3;

#[derive(Debug, Deserialize)]
struct TaskSpec {
    task: String,
    #[serde(default)]
    context: String,
    #[serde(default)]
    constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
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

const SYSTEM_PROMPT: &str = "\
You are a code-generation agent. Respond with a JSON object containing:
- \"status\": \"ok\" or \"error\"
- \"code\": the generated code as a string (if applicable)
- \"explanation\": a brief explanation of what you did
- \"files_modified\": an array of file paths modified (empty if not applicable)

Respond ONLY with valid JSON. No markdown fences, no commentary outside the JSON.";

fn build_user_message(spec: &TaskSpec) -> String {
    let mut parts = vec![format!("Task: {}", spec.task)];

    if !spec.context.is_empty() {
        parts.push(format!("Context:\n{}", spec.context));
    }

    if !spec.constraints.is_empty() {
        let list = spec
            .constraints
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n");
        parts.push(format!("Constraints:\n{list}"));
    }

    parts.join("\n\n")
}

/// Validate that a parsed JSON value has the required dispatch response fields.
fn validate_response(v: &Value) -> Result<(), String> {
    let obj = v
        .as_object()
        .ok_or_else(|| "response is not a JSON object".to_string())?;

    if !obj.contains_key("status") {
        return Err("missing required field: \"status\"".to_string());
    }
    if !obj.contains_key("code") {
        return Err("missing required field: \"code\"".to_string());
    }
    if !obj.contains_key("explanation") {
        return Err("missing required field: \"explanation\"".to_string());
    }
    if !obj.contains_key("files_modified") {
        return Err("missing required field: \"files_modified\"".to_string());
    }

    Ok(())
}

pub fn run_capture(level: u8, input: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Try parsing stdin as-is; if that fails due to control characters,
    // normalize bare newlines/tabs inside JSON string values and retry.
    let spec: TaskSpec = serde_json::from_str(input)
        .or_else(|_| {
            let sanitized = sanitize_json_strings(input);
            serde_json::from_str(&sanitized)
        })
        .map_err(|e| format!("invalid JSON on stdin: {e}"))?;

    let user_message = build_user_message(&spec);
    let base_url = defaults::configured_ollama_base_url();
    let url = defaults::ollama_chat_completions_url(&base_url);
    let model = defaults::configured_model_for_level(level)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let max_tokens = defaults::max_tokens_for_level(level)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let initial_request = ChatRequest {
        model: model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_message,
            },
        ],
        max_tokens,
        temperature: 0.2,
        stream: false,
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let client = reqwest::Client::new();
        let output = dispatch_with_retry(&client, &url, initial_request).await?;
        Ok(serde_json::to_string_pretty(&output)?)
    })
}

pub fn run(level: u8, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = run_capture(level, input)?;
    println!("{output}");
    Ok(())
}

/// Send a dispatch request with retry-on-failure and error feedback.
/// Retries up to MAX_RETRIES times, appending the parse error as a user
/// correction message so the model can self-correct its output format.
async fn dispatch_with_retry(
    client: &reqwest::Client,
    url: &str,
    initial_request: ChatRequest,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut request = initial_request;
    let mut last_error = String::new();

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            // Append error feedback so the model can self-correct.
            eprintln!("dispatch: attempt {attempt}/{MAX_RETRIES}, last error: {last_error}");
            request.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: last_error.clone(), // echo back the bad output
            });
            request.messages.push(ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Your response was invalid: {last_error}\n\
                     Respond ONLY with valid JSON containing: \
                     status, code, explanation, files_modified. \
                     No markdown fences. No text outside the JSON object."
                ),
            });
        }

        let raw_text = match send_request(client, url, &request).await {
            Ok(text) => text,
            Err(e) => {
                // Network/Ollama errors are not retryable.
                return Err(e);
            }
        };

        let text = strip_code_fences(&raw_text);

        match serde_json::from_str::<Value>(&text) {
            Ok(value) => match validate_response(&value) {
                Ok(()) => return Ok(value),
                Err(e) => {
                    last_error = format!("schema error: {e}");
                }
            },
            Err(e) => {
                // Include the first 200 chars of bad output so the model sees what went wrong.
                let preview: String = raw_text.chars().take(200).collect();
                last_error = format!("JSON parse error: {e}. Your output began with: {preview:?}");
            }
        }
    }

    // All retries exhausted — return a structured error.
    eprintln!("dispatch: all {MAX_RETRIES} retries exhausted");
    Ok(serde_json::json!({
        "status": "error",
        "code": "",
        "explanation": format!("Failed to get valid JSON after {} attempts. Last error: {}", MAX_RETRIES + 1, last_error),
        "files_modified": []
    }))
}

/// Send a single HTTP request to the Ollama API and return the raw response text.
async fn send_request(
    client: &reqwest::Client,
    url: &str,
    request: &ChatRequest,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = client.post(url).json(request).send().await.map_err(|e| {
        format!(
            "failed to reach Ollama at {url}: {e}\n\
                 Is Ollama running? Try: ollama serve"
        )
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unreadable>".to_string());
        return Err(format!("Ollama returned {status}: {body}").into());
    }

    let chat_response: ChatResponse = response
        .json()
        .await
        .map_err(|e| format!("failed to parse Ollama response: {e}"))?;

    Ok(chat_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default())
}
