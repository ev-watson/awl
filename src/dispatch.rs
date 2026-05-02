#![allow(
    clippy::doc_markdown,
    clippy::format_push_string,
    clippy::too_many_lines
)]

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config;
use crate::defaults;
use crate::llm_io::{sanitize_json_strings, strip_code_fences};
use crate::safety;

const FORMAT_RETRIES: usize = 3;
const DEFAULT_MAX_RETURN_CHARS: usize = 4_000;
const DEFAULT_CONTEXT_FILE_CHARS: usize = 8_000;
const DEFAULT_TOTAL_CONTEXT_CHARS: usize = 24_000;
const DEFAULT_FAILURE_ISSUE_CHARS: usize = 700;
const VERIFY_TIMEOUT_MS: u64 = 120_000;

#[derive(Debug, Clone)]
pub struct DispatchOptions {
    pub level: u8,
    pub apply: bool,
    pub verify_command: Option<String>,
    pub target_path: Option<String>,
    pub max_attempts: Option<usize>,
    pub max_return_chars: Option<usize>,
    pub auto_repomap: bool,
    pub repomap_focus: Vec<String>,
    pub repomap_budget: Option<usize>,
    pub model: Option<String>,
}

impl DispatchOptions {
    pub fn new(level: u8) -> Self {
        Self {
            level,
            apply: false,
            verify_command: None,
            target_path: None,
            max_attempts: None,
            max_return_chars: None,
            auto_repomap: false,
            repomap_focus: Vec::new(),
            repomap_budget: None,
            model: None,
        }
    }
}

impl DispatchLog {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let dir = config::config_dir()?.join("dispatches");
        fs::create_dir_all(&dir)?;
        let id = dispatch_id();
        let path = dir.join(format!("{id}.jsonl"));
        Ok(Self { id, path })
    }

    fn append(&self, event: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        serde_json::to_writer(&mut file, event)?;
        writeln!(file)?;
        Ok(())
    }

    fn path_display(&self) -> String {
        self.path.display().to_string()
    }
}

fn dispatch_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("{nanos}-{}", std::process::id())
}

fn dispatches_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(config::config_dir()?.join("dispatches"))
}

fn dispatch_log_path(id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("dispatch id must be a file name, not a path".into());
    }
    Ok(dispatches_dir()?.join(format!("{id}.jsonl")))
}

fn list_dispatch_logs() -> Result<(), Box<dyn std::error::Error>> {
    let dir = dispatches_dir()?;
    if !dir.exists() {
        println!("no dispatch logs found");
        return Ok(());
    }
    let mut entries = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("jsonl") {
                return None;
            }
            let id = path.file_stem()?.to_str()?.to_string();
            let metadata = entry.metadata().ok()?;
            Some((id, metadata.len(), metadata.modified().ok()))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| b.0.cmp(&a.0));

    if entries.is_empty() {
        println!("no dispatch logs found");
        return Ok(());
    }

    println!("{:<45} {:>8}  MODIFIED", "DISPATCH ID", "SIZE");
    for (id, size, modified) in entries {
        let age = modified.and_then(|time| time.elapsed().ok()).map_or_else(
            || "unknown".to_string(),
            |duration| format!("{}s ago", duration.as_secs()),
        );
        println!("{id:<45} {size:>8}  {age}");
    }
    Ok(())
}

fn prune_dispatch_logs(max_age_days: u64) -> Result<usize, Box<dyn std::error::Error>> {
    prune_dispatch_logs_in(&dispatches_dir()?, max_age_days)
}

fn prune_dispatch_logs_in(
    dir: &Path,
    max_age_days: u64,
) -> Result<usize, Box<dyn std::error::Error>> {
    if !dir.exists() {
        return Ok(0);
    }

    let cutoff = SystemTime::now() - Duration::from_secs(max_age_days * 24 * 60 * 60);
    let mut deleted = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("jsonl") {
            continue;
        }

        let metadata = entry.metadata()?;
        if metadata.modified()? < cutoff {
            fs::remove_file(&path)?;
            deleted += 1;
        }
    }
    Ok(deleted)
}

fn show_dispatch_log(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = dispatch_log_path(id)?;
    print!("{}", fs::read_to_string(&path)?);
    Ok(())
}

fn tail_dispatch_log(id: &str, count: usize) -> Result<(), Box<dyn std::error::Error>> {
    let path = dispatch_log_path(id)?;
    let raw = fs::read_to_string(&path)?;
    let lines = raw.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(count);
    for line in &lines[start..] {
        println!("{line}");
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
struct TaskSpec {
    task: String,
    #[serde(default)]
    context: String,
    #[serde(default)]
    constraints: Vec<String>,
    #[serde(default, alias = "target_file")]
    target_path: Option<String>,
    #[serde(default)]
    target_files: Vec<String>,
    #[serde(default)]
    context_paths: Vec<String>,
    #[serde(default)]
    verify_command: Option<String>,
    #[serde(default)]
    apply: Option<bool>,
    #[serde(default)]
    max_attempts: Option<usize>,
    #[serde(default)]
    max_return_chars: Option<usize>,
    #[serde(default)]
    auto_repomap: bool,
    #[serde(default)]
    repomap_focus: Vec<String>,
    #[serde(default)]
    repomap_budget: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f64,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

struct GeneratedDispatch {
    value: Value,
    usage: Option<Value>,
}

struct DispatchLog {
    id: String,
    path: PathBuf,
}

struct FileSnapshot {
    path: PathBuf,
    previous_contents: Option<Vec<u8>>,
}

struct VerifyResult {
    success: bool,
    output: String,
}

struct ApplyFlowConfig {
    target_path: String,
    verify_command: Option<String>,
    max_attempts: usize,
    max_return_chars: usize,
}

const SYSTEM_PROMPT: &str = "\
You are a bounded code-generation worker. Respond with a JSON object containing:
- \"status\": \"ok\" or \"error\"
- \"code\": the generated code as a string
- \"explanation\": a brief explanation of what you did
- \"files_modified\": an array of file paths modified, or intended to be modified

Use status \"ok\" when you can satisfy the requested coding task. Use status \"error\" only \
when the request is impossible with the provided context. The response must match this JSON \
schema exactly:
{
  \"type\": \"object\",
  \"additionalProperties\": false,
  \"properties\": {
    \"status\": {\"type\": \"string\", \"enum\": [\"ok\", \"error\"]},
    \"code\": {\"type\": \"string\"},
    \"explanation\": {\"type\": \"string\"},
    \"files_modified\": {\"type\": \"array\", \"items\": {\"type\": \"string\"}}
  },
  \"required\": [\"status\", \"code\", \"explanation\", \"files_modified\"]
}

When a target path is provided, return the complete file contents for exactly that path in \
\"code\". Do not claim that files were written; Awl will write and verify them. Respond ONLY \
with valid JSON. No markdown fences, no commentary outside the JSON.";

fn build_user_message(
    spec: &TaskSpec,
    target_path: Option<&str>,
    verify_command: Option<&str>,
    repo_map: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut parts = vec![format!("Task: {}", spec.task)];

    if let Some(path) = target_path {
        parts.push(format!("Target path:\n{path}"));
    }

    if !spec.target_files.is_empty() {
        parts.push(format!("Target files:\n{}", spec.target_files.join("\n")));
    }

    if !spec.context.is_empty() {
        parts.push(format!("Context:\n{}", spec.context));
    }

    if !spec.context_paths.is_empty() {
        parts.push(format!(
            "Context from files:\n{}",
            read_context_paths(&spec.context_paths)?
        ));
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

    if let Some(command) = verify_command {
        parts.push(format!(
            "Acceptance check Awl will run after applying the result:\n{command}"
        ));
    }

    if let Some(map) = repo_map {
        parts.push(format!("Local repository map:\n{map}"));
    }

    Ok(parts.join("\n\n"))
}

fn read_context_paths(paths: &[String]) -> Result<String, Box<dyn std::error::Error>> {
    let mut out = String::new();
    let mut total_chars = 0usize;

    for path in paths {
        if total_chars >= DEFAULT_TOTAL_CONTEXT_CHARS {
            let _ = writeln!(out, "\n[context truncated]");
            break;
        }

        let resolved = safety::resolve_existing_path(Path::new(path))?;
        let raw = fs::read_to_string(&resolved)
            .map_err(|e| format!("failed reading context file {}: {e}", resolved.display()))?;
        let remaining = DEFAULT_TOTAL_CONTEXT_CHARS.saturating_sub(total_chars);
        let limit = DEFAULT_CONTEXT_FILE_CHARS.min(remaining);
        let content = truncate_owned(&raw, limit);
        total_chars += content.len();

        let _ = writeln!(out, "--- {path} ---");
        let _ = writeln!(out, "{content}");
    }

    Ok(out)
}

fn preflight(spec: &TaskSpec, apply: bool, target_path: Option<&str>) -> Result<(), String> {
    for path in &spec.context_paths {
        safety::resolve_existing_path(Path::new(path))
            .map_err(|error| format!("context path `{path}` is invalid: {error}"))?;
    }

    if apply {
        if target_path.is_none() {
            return Err(
                "apply mode requires target_path or exactly one target_files entry".to_string(),
            );
        }
        if spec.target_path.is_none() && spec.target_files.len() > 1 {
            return Err(
                "apply mode cannot infer one write target from multiple target_files entries"
                    .to_string(),
            );
        }
    }

    if let Some(path) = target_path {
        safety::resolve_path_for_write(Path::new(path))
            .map_err(|error| format!("target path `{path}` is invalid: {error}"))?;
    }

    Ok(())
}

/// Scan generated code for `use crate::IDENT` paths whose first segment is
/// not in `known_modules`. Returns the unresolved bare identifiers, deduped
/// and order-preserving. Only runs for `.rs` targets; other languages return
/// an empty vec (no preflight).
///
/// This is a fast pre-write check: if the model invents a crate-internal
/// module that doesn't exist, we skip the file write entirely and feed the
/// names back to the model rather than burning a verify cycle (or worse,
/// writing broken code in apply-without-verify mode).
fn unresolved_crate_imports(
    target_path: &str,
    code: &str,
    known_modules: &HashSet<String>,
) -> Vec<String> {
    if !Path::new(target_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
    {
        return Vec::new();
    }

    let mut unresolved: Vec<String> = Vec::new();
    let needle = "use crate::";
    let mut search = code;
    while let Some(start) = search.find(needle) {
        let after = &search[start + needle.len()..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let ident = &after[..end];
        if !ident.is_empty()
            && !known_modules.contains(ident)
            && !unresolved.iter().any(|u| u == ident)
        {
            unresolved.push(ident.to_string());
        }
        search = &after[end..];
    }
    unresolved
}

fn build_repo_map_context(spec: &TaskSpec, target_path: Option<&str>) -> Option<String> {
    if !spec.auto_repomap {
        return None;
    }

    let mut focus = Vec::new();
    if let Some(path) = target_path {
        focus.push(PathBuf::from(path));
    }
    focus.extend(spec.target_files.iter().map(PathBuf::from));
    focus.extend(spec.context_paths.iter().map(PathBuf::from));
    focus.extend(spec.repomap_focus.iter().map(PathBuf::from));
    focus.sort();
    focus.dedup();

    let budget = spec.repomap_budget.unwrap_or(1200).clamp(200, 4096);
    match crate::repomap::generate(Path::new("."), budget, &focus) {
        Ok(map) => Some(map),
        Err(error) => Some(format!(
            "# Repository Map\n\nUnavailable because repomap failed locally: {error}"
        )),
    }
}

/// Validate that a parsed JSON value has the required dispatch response fields.
fn validate_response(value: &Value) -> Result<(), String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "response is not a JSON object".to_string())?;

    if !matches!(
        obj.get("status").and_then(Value::as_str),
        Some("ok" | "error")
    ) {
        return Err("missing or invalid required field: \"status\"".to_string());
    }
    if !obj.get("code").is_some_and(Value::is_string) {
        return Err("missing or invalid required field: \"code\"".to_string());
    }
    if !obj.get("explanation").is_some_and(Value::is_string) {
        return Err("missing or invalid required field: \"explanation\"".to_string());
    }
    if !obj.get("files_modified").is_some_and(Value::is_array) {
        return Err("missing or invalid required field: \"files_modified\"".to_string());
    }

    Ok(())
}

pub fn run_capture(
    options: &DispatchOptions,
    input: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Try parsing stdin as-is; if that fails due to control characters,
    // normalize bare newlines/tabs inside JSON string values and retry.
    let mut spec: TaskSpec = serde_json::from_str(input)
        .or_else(|_| {
            let sanitized = sanitize_json_strings(input);
            serde_json::from_str(&sanitized)
        })
        .map_err(|e| format!("invalid JSON on stdin: {e}"))?;

    let apply = options.apply || spec.apply.unwrap_or(false);
    let target_path = effective_target_path(options.target_path.as_deref(), &spec);
    let verify_command = options
        .verify_command
        .clone()
        .or_else(|| spec.verify_command.clone());
    if let Some(command) = &verify_command {
        safety::validate_shell_command(command)?;
    }
    let max_return_chars = options
        .max_return_chars
        .or(spec.max_return_chars)
        .unwrap_or(DEFAULT_MAX_RETURN_CHARS);
    let max_attempts = effective_max_attempts(
        options.max_attempts.or(spec.max_attempts),
        apply,
        verify_command.is_some(),
    );

    if options.auto_repomap {
        spec.auto_repomap = true;
    }
    if !options.repomap_focus.is_empty() {
        spec.repomap_focus.extend(options.repomap_focus.clone());
    }
    if options.repomap_budget.is_some() {
        spec.repomap_budget = options.repomap_budget;
    }

    let dispatch_log = DispatchLog::new()?;
    dispatch_log.append(&json!({
        "event": "dispatch_start",
        "level": options.level,
        "apply": apply,
        "target_path": target_path.as_deref(),
        "verify_command": verify_command.as_deref(),
        "auto_repomap": spec.auto_repomap
    }))?;
    let started = Instant::now();
    if let Err(error) = preflight(&spec, apply, target_path.as_deref()) {
        dispatch_log.append(&json!({
            "event": "preflight_failed",
            "error": error
        }))?;
        let mut output = error_result(&error, &[], 0, Some("preflight"));
        add_top_level_telemetry(
            &mut output,
            "",
            options.level,
            elapsed_ms(started),
            &dispatch_log,
        );
        return Ok(serde_json::to_string_pretty(&output)?);
    }

    let repo_map = build_repo_map_context(&spec, target_path.as_deref());
    if let Some(map) = &repo_map {
        dispatch_log.append(&json!({
            "event": "repomap_injected",
            "chars": map.len(),
            "budget": spec.repomap_budget.unwrap_or(1200)
        }))?;
    }

    let user_message = build_user_message(
        &spec,
        target_path.as_deref(),
        verify_command.as_deref(),
        repo_map.as_deref(),
    )?;
    let base_url = defaults::configured_ollama_base_url();
    let url = defaults::ollama_chat_completions_url(&base_url);
    let model = match &options.model {
        Some(m) => m.clone(),
        None => defaults::configured_model_for_level(options.level)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?,
    };
    let max_tokens = defaults::max_tokens_for_level(options.level)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let model_name = model.clone();
    dispatch_log.append(&json!({
        "event": "model_selected",
        "model": &model_name
    }))?;
    let initial_request = ChatRequest {
        model,
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
        temperature: 0.0,
        stream: false,
        response_format: Some(dispatch_response_format()),
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let client = reqwest::Client::new();
        if apply {
            let Some(target_path) = target_path else {
                dispatch_log.append(&json!({
                    "event": "apply_missing_target_path"
                }))?;
                let mut output = error_result(
                    "apply mode requires target_path or exactly one target_files entry",
                    &["missing target path".to_string()],
                    0,
                    Some("preflight"),
                );
                add_top_level_telemetry(
                    &mut output,
                    &model_name,
                    options.level,
                    elapsed_ms(started),
                    &dispatch_log,
                );
                return Ok(serde_json::to_string_pretty(&output)?);
            };

            let mut output = run_apply_flow(
                &client,
                &url,
                initial_request,
                &ApplyFlowConfig {
                    target_path: target_path.clone(),
                    verify_command,
                    max_attempts,
                    max_return_chars,
                },
                &dispatch_log,
            )
            .await?;
            add_top_level_telemetry(
                &mut output,
                &model_name,
                options.level,
                elapsed_ms(started),
                &dispatch_log,
            );
            Ok(serde_json::to_string_pretty(&output)?)
        } else {
            let generated =
                match dispatch_with_retry(&client, &url, initial_request, &dispatch_log).await {
                    Ok(generated) => generated,
                    Err(error) => {
                        let summary = error.to_string();
                        let mut output = error_result(&summary, &[], 1, Some("network"));
                        add_top_level_telemetry(
                            &mut output,
                            &model_name,
                            options.level,
                            elapsed_ms(started),
                            &dispatch_log,
                        );
                        compact_value_for_return(&mut output, max_return_chars, false);
                        return Ok(serde_json::to_string_pretty(&output)?);
                    }
                };
            let mut output = generated.value;
            if let Some(usage) = generated.usage {
                output["usage"] = usage;
            }
            normalize_non_apply_output(&mut output);
            add_top_level_telemetry(
                &mut output,
                &model_name,
                options.level,
                elapsed_ms(started),
                &dispatch_log,
            );
            compact_value_for_return(&mut output, max_return_chars, false);
            Ok(serde_json::to_string_pretty(&output)?)
        }
    })
}

fn dispatch_response_format() -> Value {
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "dispatch_response",
            "strict": true,
            "schema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "status": {"type": "string", "enum": ["ok", "error"]},
                    "code": {"type": "string"},
                    "explanation": {"type": "string"},
                    "files_modified": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["status", "code", "explanation", "files_modified"]
            }
        }
    })
}

pub fn run(options: &DispatchOptions, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = run_capture(options, input)?;
    println!("{output}");
    Ok(())
}

pub fn run_logs(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args.first().map_or("--list", String::as_str) {
        "--list" | "list" => list_dispatch_logs(),
        "--show" | "show" => {
            let id = args.get(1).ok_or("dispatches --show requires an id")?;
            show_dispatch_log(id)
        }
        "--tail" | "tail" => {
            let id = args.get(1).ok_or("dispatches --tail requires an id")?;
            tail_dispatch_log(id, 20)
        }
        "--prune" | "prune" => {
            let days: u64 = args
                .get(1)
                .ok_or("dispatches --prune requires a number of days")?
                .parse()
                .map_err(|_| "dispatches --prune value must be a positive integer")?;
            let deleted = prune_dispatch_logs(days)?;
            println!("deleted {deleted} dispatch log(s)");
            Ok(())
        }
        "--help" | "-h" | "help" => {
            println!(
                "Usage:\n  awl dispatches [--list]\n  awl dispatches --show <dispatch-id>\n  awl dispatches --tail <dispatch-id>\n  awl dispatches --prune <days>"
            );
            Ok(())
        }
        other => Err(format!(
            "unknown dispatches flag: {other}\n\nUsage:\n  awl dispatches [--list]\n  awl dispatches --show <dispatch-id>\n  awl dispatches --tail <dispatch-id>\n  awl dispatches --prune <days>"
        )
        .into()),
    }
}

async fn run_apply_flow(
    client: &reqwest::Client,
    url: &str,
    initial_request: ChatRequest,
    config: &ApplyFlowConfig,
    dispatch_log: &DispatchLog,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut request = initial_request;
    let mut issues = Vec::new();
    let mut usage = Vec::new();
    let mut last_failure_category: Option<&'static str> = None;

    for attempt in 1..=config.max_attempts {
        let generated = match dispatch_with_retry(client, url, request.clone(), dispatch_log).await
        {
            Ok(generated) => generated,
            Err(error) => {
                let summary = error.to_string();
                issues.push(summary.clone());
                dispatch_log.append(&json!({
                    "event": "network_error",
                    "attempt": attempt,
                    "error": summary
                }))?;
                return Ok(apply_result(
                    "error",
                    &summary,
                    &[],
                    config.verify_command.as_deref(),
                    false,
                    attempt,
                    &usage,
                    &issues,
                    Some("network"),
                ));
            }
        };
        if let Some(last_usage) = generated.usage.clone() {
            usage.push(last_usage);
        }

        if status(&generated.value) != Some("ok") {
            let summary = generated
                .value
                .get("explanation")
                .and_then(Value::as_str)
                .unwrap_or("model returned error");
            issues.push(summary.to_string());
            dispatch_log.append(&json!({
                "event": "model_status_error",
                "attempt": attempt,
                "summary": summary
            }))?;
            let failure_category = result_failure_category(&generated.value).unwrap_or("model");
            return Ok(apply_result(
                "error",
                summary,
                &[],
                config.verify_command.as_deref(),
                false,
                attempt,
                &usage,
                &issues,
                Some(failure_category),
            ));
        }

        let Some(code) = generated.value.get("code").and_then(Value::as_str) else {
            issues.push("model response did not include code".to_string());
            dispatch_log.append(&json!({
                "event": "missing_code",
                "attempt": attempt
            }))?;
            return Ok(apply_result(
                "error",
                "model response did not include code",
                &[],
                config.verify_command.as_deref(),
                false,
                attempt,
                &usage,
                &issues,
                Some("schema"),
            ));
        };

        let known_modules = crate::repomap::known_rust_modules(Path::new("."));
        let unresolved = unresolved_crate_imports(&config.target_path, code, &known_modules);
        if !unresolved.is_empty() {
            dispatch_log.append(&json!({
                "event": "preflight_unresolved_imports",
                "attempt": attempt,
                "target_path": &config.target_path,
                "unresolved": &unresolved
            }))?;
            let issue = format!(
                "preflight on attempt {attempt}: unresolved crate-internal imports: {}",
                unresolved.join(", ")
            );
            issues.push(issue);

            if attempt >= config.max_attempts {
                return Ok(apply_result(
                    "error",
                    "preflight detected unresolved crate-internal imports; nothing was written",
                    &[],
                    config.verify_command.as_deref(),
                    false,
                    attempt,
                    &usage,
                    &issues,
                    Some("preflight"),
                ));
            }

            request.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: model_feedback_summary(&generated.value, config.max_return_chars),
            });
            request.messages.push(ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Awl did not write the file. The generated code references crate-internal \
                     modules that do not exist in this repo: {}. Either use a real local module, \
                     import from a real external crate, or restructure to avoid these paths. \
                     Return the corrected complete file as valid JSON again.",
                    unresolved.join(", ")
                ),
            });
            continue;
        }

        let snapshot = capture_snapshot(&config.target_path)?;
        write_target(&config.target_path, code)?;
        dispatch_log.append(&json!({
            "event": "file_written",
            "attempt": attempt,
            "target_path": &config.target_path
        }))?;

        if let Some(command) = &config.verify_command {
            let check = match run_verify_command(command) {
                Ok(check) => check,
                Err(error) => {
                    restore_snapshot(snapshot)?;
                    dispatch_log.append(&json!({
                        "event": "verify_command_error",
                        "attempt": attempt,
                        "target_path": &config.target_path,
                        "error": error.to_string(),
                        "rollback": "restored"
                    }))?;
                    issues.push(format!("verify command failed to run: {error}"));
                    return Ok(apply_result(
                        "error",
                        "verify command failed to run; previous file contents were restored",
                        &[],
                        Some(command),
                        false,
                        attempt,
                        &usage,
                        &issues,
                        Some("verify"),
                    ));
                }
            };
            if check.success {
                dispatch_log.append(&json!({
                    "event": "verify_passed",
                    "attempt": attempt,
                    "target_path": &config.target_path,
                    "command": command
                }))?;
                let summary = generated
                    .value
                    .get("explanation")
                    .and_then(Value::as_str)
                    .unwrap_or("applied and verified");
                return Ok(apply_result(
                    "ok",
                    summary,
                    std::slice::from_ref(&config.target_path),
                    Some(command),
                    true,
                    attempt,
                    &usage,
                    &[],
                    None,
                ));
            }

            restore_snapshot(snapshot)?;
            let failure_category = verify_failure_category(&check.output);
            last_failure_category = Some(failure_category);
            dispatch_log.append(&json!({
                "event": "verify_failed",
                "attempt": attempt,
                "target_path": &config.target_path,
                "command": command,
                "output": &check.output,
                "rollback": "restored"
            }))?;
            let issue = format!(
                "verify failed on attempt {attempt}: {}",
                truncate(&check.output, failure_issue_limit(config.max_return_chars))
            );
            issues.push(issue.clone());

            request.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: model_feedback_summary(&generated.value, config.max_return_chars),
            });
            request.messages.push(ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Awl applied your code to {} and ran `{command}`. Verification failed, \
                     so Awl restored the previous file contents. Fix the code and return the \
                     complete replacement file as valid JSON again.\n\nVerifier output:\n{}",
                    config.target_path,
                    truncate(&check.output, config.max_return_chars)
                ),
            });
        } else {
            dispatch_log.append(&json!({
                "event": "apply_without_verify",
                "attempt": attempt,
                "target_path": &config.target_path
            }))?;
            let summary = generated
                .value
                .get("explanation")
                .and_then(Value::as_str)
                .unwrap_or("applied");
            return Ok(apply_result(
                "ok",
                summary,
                std::slice::from_ref(&config.target_path),
                None,
                false,
                attempt,
                &usage,
                &[],
                None,
            ));
        }
    }

    Ok(apply_result(
        "error",
        "verification failed after all attempts; previous file contents were restored",
        &[],
        config.verify_command.as_deref(),
        false,
        config.max_attempts,
        &usage,
        &issues,
        Some(last_failure_category.unwrap_or("verify")),
    ))
}

/// Send a dispatch request with retry-on-format-failure and error feedback.
async fn dispatch_with_retry(
    client: &reqwest::Client,
    url: &str,
    initial_request: ChatRequest,
    dispatch_log: &DispatchLog,
) -> Result<GeneratedDispatch, Box<dyn std::error::Error>> {
    let mut request = initial_request;
    let mut last_error = String::new();
    let mut last_usage = None;

    for attempt in 0..=FORMAT_RETRIES {
        if attempt > 0 {
            eprintln!("dispatch: attempt {attempt}/{FORMAT_RETRIES}, last error: {last_error}");
            request.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: last_error.clone(),
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

        let reply = send_request(client, url, &request).await?;
        last_usage.clone_from(&reply.usage);
        let text = strip_code_fences(&reply.content);

        match serde_json::from_str::<Value>(&text) {
            Ok(value) => match validate_response(&value) {
                Ok(()) => {
                    dispatch_log.append(&json!({
                        "event": "model_response_valid",
                        "format_attempt": attempt + 1,
                        "raw_content": &reply.content,
                        "parsed": &value,
                        "usage": &reply.usage
                    }))?;
                    return Ok(GeneratedDispatch {
                        value,
                        usage: reply.usage,
                    });
                }
                Err(error) => {
                    last_error = format!("schema error: {error}");
                    dispatch_log.append(&json!({
                        "event": "model_response_invalid_schema",
                        "format_attempt": attempt + 1,
                        "error": error,
                        "raw_content": &reply.content,
                        "usage": &reply.usage
                    }))?;
                }
            },
            Err(error) => {
                let preview: String = reply.content.chars().take(200).collect();
                last_error =
                    format!("JSON parse error: {error}. Your output began with: {preview:?}");
                dispatch_log.append(&json!({
                    "event": "model_response_invalid_json",
                    "format_attempt": attempt + 1,
                    "error": error.to_string(),
                    "raw_content": &reply.content,
                    "usage": &reply.usage
                }))?;
            }
        }
    }

    eprintln!("dispatch: all {FORMAT_RETRIES} retries exhausted");
    let value = json!({
        "status": "error",
        "code": "",
        "explanation": format!(
            "Failed to get valid JSON after {} attempts. Last error: {}",
            FORMAT_RETRIES + 1,
            last_error
        ),
        "files_modified": [],
        "failure_category": "format"
    });
    dispatch_log.append(&json!({
        "event": "format_retries_exhausted",
        "error": last_error,
        "usage": &last_usage
    }))?;
    Ok(GeneratedDispatch {
        value,
        usage: last_usage,
    })
}

struct ModelReply {
    content: String,
    usage: Option<Value>,
}

/// Send a single HTTP request to the Ollama API and return the raw response text.
async fn send_request(
    client: &reqwest::Client,
    url: &str,
    request: &ChatRequest,
) -> Result<ModelReply, Box<dyn std::error::Error>> {
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

    Ok(ModelReply {
        content: chat_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_default(),
        usage: chat_response.usage,
    })
}

fn effective_target_path(cli_target_path: Option<&str>, spec: &TaskSpec) -> Option<String> {
    cli_target_path
        .map(ToString::to_string)
        .or_else(|| spec.target_path.clone())
        .or_else(|| {
            if spec.target_files.len() == 1 {
                spec.target_files.first().cloned()
            } else {
                None
            }
        })
}

fn effective_max_attempts(raw: Option<usize>, apply: bool, has_verify: bool) -> usize {
    let _ = (apply, has_verify);
    let default = 1;
    raw.unwrap_or(default).clamp(1, 5)
}

fn status(value: &Value) -> Option<&str> {
    value.get("status").and_then(Value::as_str)
}

fn capture_snapshot(path: &str) -> Result<FileSnapshot, Box<dyn std::error::Error>> {
    let resolved = safety::resolve_path_for_write(Path::new(path))?;
    let previous_contents = match fs::read(&resolved) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "failed to snapshot {} before dispatch apply: {error}",
                resolved.display()
            )
            .into());
        }
    };
    Ok(FileSnapshot {
        path: resolved,
        previous_contents,
    })
}

fn write_target(path: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let resolved = safety::resolve_path_for_write(Path::new(path))?;
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed creating parent dir {}: {e}", parent.display()))?;
    }
    fs::write(&resolved, content)
        .map_err(|e| format!("failed writing {}: {e}", resolved.display()))?;
    Ok(())
}

fn restore_snapshot(snapshot: FileSnapshot) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(contents) = snapshot.previous_contents {
        fs::write(&snapshot.path, contents)
            .map_err(|e| format!("failed restoring {}: {e}", snapshot.path.display()))?;
    } else if snapshot.path.exists() {
        fs::remove_file(&snapshot.path)
            .map_err(|e| format!("failed removing {}: {e}", snapshot.path.display()))?;
    }
    Ok(())
}

fn run_verify_command(command: &str) -> Result<VerifyResult, Box<dyn std::error::Error>> {
    safety::validate_shell_command(command)?;
    let workspace = safety::workspace_root()?;
    let timeout = Duration::from_millis(VERIFY_TIMEOUT_MS);
    let mut child = std::process::Command::new("bash")
        .arg("-lc")
        .arg(command)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to run verify command: {e}"))?;
    let deadline = Instant::now() + timeout;

    let output = loop {
        match child.try_wait().map_err(|e| format!("wait failed: {e}"))? {
            Some(status) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut stream) = child.stdout.take() {
                    std::io::Read::read_to_end(&mut stream, &mut stdout).ok();
                }
                if let Some(mut stream) = child.stderr.take() {
                    std::io::Read::read_to_end(&mut stream, &mut stderr).ok();
                }
                break std::process::Output {
                    status,
                    stdout,
                    stderr,
                };
            }
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                return Ok(VerifyResult {
                    success: false,
                    output: format!("verify command timed out after {VERIFY_TIMEOUT_MS}ms"),
                });
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    };

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(VerifyResult {
        success: output.status.success(),
        output: truncate_owned(&combined, DEFAULT_MAX_RETURN_CHARS),
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_result(
    status: &str,
    summary: &str,
    files_changed: &[String],
    verify_command: Option<&str>,
    checks_passed: bool,
    attempts: usize,
    usage: &[Value],
    open_issues: &[String],
    failure_category: Option<&str>,
) -> Value {
    let checks_run = verify_command
        .map(|command| vec![command.to_string()])
        .unwrap_or_default();
    let compact_issues = compact_issues(open_issues, DEFAULT_FAILURE_ISSUE_CHARS);
    json!({
        "status": status,
        "summary": truncate(summary, DEFAULT_MAX_RETURN_CHARS),
        "files_changed": files_changed,
        "checks_run": checks_run,
        "checks_passed": checks_passed,
        "attempts": attempts,
        "usage": usage,
        "failure_category": failure_category,
        "open_issues": compact_issues,
        "code": "",
        "explanation": truncate(summary, DEFAULT_MAX_RETURN_CHARS),
        "files_modified": files_changed
    })
}

fn error_result(
    summary: &str,
    open_issues: &[String],
    attempts: usize,
    failure_category: Option<&str>,
) -> Value {
    let issues = if open_issues.is_empty() {
        vec![summary.to_string()]
    } else {
        open_issues.to_vec()
    };
    json!({
        "status": "error",
        "summary": truncate(summary, DEFAULT_MAX_RETURN_CHARS),
        "files_changed": [],
        "checks_run": [],
        "checks_passed": false,
        "attempts": attempts,
        "usage": [],
        "failure_category": failure_category,
        "open_issues": compact_issues(&issues, DEFAULT_FAILURE_ISSUE_CHARS),
        "code": "",
        "explanation": truncate(summary, DEFAULT_MAX_RETURN_CHARS),
        "files_modified": []
    })
}

fn normalize_non_apply_output(output: &mut Value) {
    let files_intended = output
        .get("files_modified")
        .cloned()
        .filter(Value::is_array)
        .unwrap_or_else(|| json!([]));
    output["files_intended"] = files_intended;
    output["files_changed"] = json!([]);
    output["checks_run"] = json!([]);
    output["checks_passed"] = json!(false);
    output["attempts"] = json!(1);
    output["failure_category"] = if status(output) == Some("ok") {
        json!(null)
    } else {
        output
            .get("failure_category")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or_else(|| json!("unknown"))
    };
    output["open_issues"] = if status(output) == Some("ok") {
        json!([])
    } else {
        json!([truncate(
            output
                .get("explanation")
                .and_then(Value::as_str)
                .unwrap_or("dispatch failed"),
            DEFAULT_FAILURE_ISSUE_CHARS
        )])
    };
    // Keep files_modified as a trusted compatibility alias: actual files changed by Awl.
    output["files_modified"] = json!([]);
}

fn result_failure_category(value: &Value) -> Option<&'static str> {
    match value.get("failure_category").and_then(Value::as_str) {
        Some("format") => Some("format"),
        Some("schema") => Some("schema"),
        Some("preflight") => Some("preflight"),
        Some("verify") => Some("verify"),
        Some("timeout") => Some("timeout"),
        Some("network") => Some("network"),
        Some("model") => Some("model"),
        Some("unknown") => Some("unknown"),
        _ => None,
    }
}

fn verify_failure_category(output: &str) -> &'static str {
    if output.to_ascii_lowercase().contains("timed out") {
        "timeout"
    } else {
        "verify"
    }
}

fn add_top_level_telemetry(
    output: &mut Value,
    model: &str,
    level: u8,
    elapsed_ms: u128,
    dispatch_log: &DispatchLog,
) {
    output["telemetry"] = json!({
        "model": model,
        "level": level,
        "elapsed_ms": elapsed_ms,
        "dispatch_id": &dispatch_log.id,
        "log_path": dispatch_log.path_display()
    });
}

fn elapsed_ms(started: Instant) -> u128 {
    started.elapsed().as_millis()
}

fn model_feedback_summary(value: &Value, max_chars: usize) -> String {
    let mut compact = value.clone();
    compact_value_for_return(&mut compact, max_chars, true);
    serde_json::to_string(&compact).unwrap_or_else(|_| "{}".to_string())
}

fn failure_issue_limit(max_return_chars: usize) -> usize {
    DEFAULT_FAILURE_ISSUE_CHARS.min(max_return_chars)
}

fn compact_issues(open_issues: &[String], max_chars_each: usize) -> Vec<String> {
    open_issues
        .iter()
        .map(|issue| {
            let truncated = truncate(issue, max_chars_each);
            if truncated.len() < issue.len() {
                format!("{truncated}\n\n[truncated; full details in dispatch log]")
            } else {
                truncated.to_string()
            }
        })
        .collect()
}

fn compact_value_for_return(value: &mut Value, max_chars: usize, applied: bool) {
    if applied {
        value["code"] = Value::String(String::new());
    } else if let Some(code_value) = value.get_mut("code") {
        if let Some(code) = code_value.as_str() {
            let truncated = truncate(code, max_chars);
            if truncated.len() < code.len() {
                *code_value = Value::String(format!("{truncated}\n\n[truncated]"));
            }
        }
    }

    if let Some(explanation_value) = value.get_mut("explanation") {
        if let Some(explanation) = explanation_value.as_str() {
            let truncated = truncate(explanation, max_chars);
            if truncated.len() < explanation.len() {
                *explanation_value = Value::String(format!("{truncated}\n\n[truncated]"));
            }
        }
    }
}

fn truncate(input: &str, max_chars: usize) -> &str {
    if input.len() <= max_chars {
        return input;
    }
    match input.char_indices().nth(max_chars) {
        Some((idx, _)) => &input[..idx],
        None => input,
    }
}

fn truncate_owned(input: &str, max_chars: usize) -> String {
    let truncated = truncate(input, max_chars);
    if truncated.len() < input.len() {
        format!("{truncated}\n\n[truncated]")
    } else {
        truncated.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock drift")
            .as_nanos();
        format!("target/awl-dispatch-tests/{name}-{nanos}.txt")
    }

    #[test]
    fn effective_target_prefers_cli_value() {
        let spec = TaskSpec {
            task: "write".to_string(),
            context: String::new(),
            constraints: Vec::new(),
            target_path: Some("from-json".to_string()),
            target_files: vec!["from-files".to_string()],
            context_paths: Vec::new(),
            verify_command: None,
            apply: None,
            max_attempts: None,
            max_return_chars: None,
            auto_repomap: false,
            repomap_focus: Vec::new(),
            repomap_budget: None,
        };

        assert_eq!(
            effective_target_path(Some("from-cli"), &spec).as_deref(),
            Some("from-cli")
        );
    }

    #[test]
    fn effective_target_uses_single_target_file() {
        let spec = TaskSpec {
            task: "write".to_string(),
            context: String::new(),
            constraints: Vec::new(),
            target_path: None,
            target_files: vec!["src/foo.rs".to_string()],
            context_paths: Vec::new(),
            verify_command: None,
            apply: None,
            max_attempts: None,
            max_return_chars: None,
            auto_repomap: false,
            repomap_focus: Vec::new(),
            repomap_budget: None,
        };

        assert_eq!(
            effective_target_path(None, &spec).as_deref(),
            Some("src/foo.rs")
        );
    }

    #[test]
    fn snapshot_restore_removes_new_dispatch_file() {
        let path = test_path("new");
        let snapshot = capture_snapshot(&path).expect("snapshot new file");
        write_target(&path, "generated").expect("write target");
        restore_snapshot(snapshot).expect("restore snapshot");
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn snapshot_restore_rewrites_existing_dispatch_file() {
        let path = test_path("existing");
        write_target(&path, "before").expect("write original");
        let snapshot = capture_snapshot(&path).expect("snapshot existing file");
        write_target(&path, "after").expect("write changed");
        restore_snapshot(snapshot).expect("restore snapshot");
        assert_eq!(fs::read_to_string(&path).expect("read restored"), "before");
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn verify_command_reports_failure_without_panicking() {
        let result = run_verify_command("python3 -c 'raise SystemExit(1)'")
            .expect("verify command should run");
        assert!(!result.success);
    }

    #[test]
    fn compact_applied_output_strips_code() {
        let mut value = json!({
            "status": "ok",
            "code": "secret generated code",
            "explanation": "done",
            "files_modified": []
        });
        compact_value_for_return(&mut value, 10, true);
        assert_eq!(value.get("code").and_then(Value::as_str), Some(""));
    }

    #[test]
    fn non_apply_output_separates_intended_from_changed_files() {
        let mut value = json!({
            "status": "ok",
            "code": "fn generated() {}",
            "explanation": "generated",
            "files_modified": ["src/generated.rs"]
        });

        normalize_non_apply_output(&mut value);

        assert_eq!(
            value
                .get("files_intended")
                .and_then(Value::as_array)
                .expect("files_intended should be present")
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>(),
            vec!["src/generated.rs"]
        );
        assert_eq!(
            value
                .get("files_changed")
                .and_then(Value::as_array)
                .expect("files_changed should be present")
                .len(),
            0
        );
        assert_eq!(
            value
                .get("files_modified")
                .and_then(Value::as_array)
                .expect("files_modified should be present")
                .len(),
            0
        );
    }

    #[test]
    fn compact_issues_points_to_dispatch_log_when_truncated() {
        let issues = vec!["x".repeat(DEFAULT_FAILURE_ISSUE_CHARS + 10)];
        let compact = compact_issues(&issues, DEFAULT_FAILURE_ISSUE_CHARS);

        assert_eq!(compact.len(), 1);
        assert!(compact[0].contains("full details in dispatch log"));
        assert!(compact[0].len() < issues[0].len() + 40);
    }

    #[test]
    fn prune_dispatch_logs_only_removes_old_jsonl_files() {
        let dir = PathBuf::from(test_path("dispatch-logs"));
        fs::create_dir_all(&dir).expect("create dispatch log test dir");
        let stale_log = dir.join("old.jsonl");
        let unrelated_file = dir.join("notes.txt");
        fs::write(&stale_log, "{}\n").expect("write stale log");
        fs::write(&unrelated_file, "keep").expect("write unrelated file");
        std::thread::sleep(Duration::from_millis(2));

        let deleted = prune_dispatch_logs_in(&dir, 0).expect("prune dispatch logs");

        assert_eq!(deleted, 1);
        assert!(!stale_log.exists());
        assert!(unrelated_file.exists());
        fs::remove_dir_all(&dir).expect("cleanup dispatch log test dir");
    }

    #[test]
    fn preflight_rejects_ambiguous_apply_targets() {
        let spec = TaskSpec {
            task: "write".to_string(),
            context: String::new(),
            constraints: Vec::new(),
            target_path: None,
            target_files: vec!["a.py".to_string(), "b.py".to_string()],
            context_paths: Vec::new(),
            verify_command: None,
            apply: Some(true),
            max_attempts: None,
            max_return_chars: None,
            auto_repomap: false,
            repomap_focus: Vec::new(),
            repomap_budget: None,
        };

        let error = preflight(&spec, true, None).expect_err("preflight should reject ambiguity");
        assert!(error.contains("requires target_path"));
    }

    #[test]
    fn unresolved_imports_flags_unknown_crate_module() {
        let mut known = HashSet::new();
        known.insert("repomap".to_string());
        known.insert("dispatch".to_string());

        let code = "use crate::repomap::generate;\n\
                    use crate::nonexistent::Thing;\n\
                    fn main() {}\n";
        let unresolved = unresolved_crate_imports("src/foo.rs", code, &known);
        assert_eq!(unresolved, vec!["nonexistent".to_string()]);
    }

    #[test]
    fn unresolved_imports_skips_non_rust_targets() {
        let known = HashSet::new();
        let code = "use crate::nonexistent::Thing;";
        let unresolved = unresolved_crate_imports("src/foo.py", code, &known);
        assert!(unresolved.is_empty());
    }

    #[test]
    fn unresolved_imports_dedupes_repeated_idents() {
        let known = HashSet::new();
        let code = "use crate::missing::A;\nuse crate::missing::B;\n";
        let unresolved = unresolved_crate_imports("src/foo.rs", code, &known);
        assert_eq!(unresolved, vec!["missing".to_string()]);
    }

    #[test]
    fn unresolved_imports_returns_empty_when_all_known() {
        let mut known = HashSet::new();
        known.insert("foo".to_string());
        known.insert("bar".to_string());
        let code = "use crate::foo::A;\nuse crate::bar::B;\nuse std::fs;\n";
        let unresolved = unresolved_crate_imports("src/x.rs", code, &known);
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_effective_max_attempts_default_one() {
        assert_eq!(effective_max_attempts(None, true, true), 1);
    }

    #[test]
    fn test_effective_max_attempts_override_two() {
        assert_eq!(effective_max_attempts(Some(2), true, true), 2);
    }

    #[test]
    fn test_effective_max_attempts_no_apply() {
        assert_eq!(effective_max_attempts(None, false, false), 1);
    }

    #[test]
    fn test_effective_max_attempts_clamp_max() {
        assert_eq!(effective_max_attempts(Some(6), true, true), 5);
    }

    #[test]
    fn test_effective_max_attempts_clamp_min() {
        assert_eq!(effective_max_attempts(Some(0), true, true), 1);
    }

    #[test]
    fn test_dispatch_options_model_default_none() {
        assert!(DispatchOptions::new(2).model.is_none());
    }

    #[test]
    fn test_apply_result_includes_failure_category() {
        let output = apply_result(
            "error",
            "verification failed",
            &[],
            Some("cargo test"),
            false,
            1,
            &[],
            &["failed".to_string()],
            Some("verify"),
        );

        assert_eq!(output["failure_category"], json!("verify"));
    }

    #[test]
    fn test_error_result_includes_failure_category() {
        let output = error_result("network unavailable", &[], 1, Some("network"));

        assert_eq!(output["failure_category"], json!("network"));
    }

    #[test]
    fn test_apply_result_success_null_category() {
        let output = apply_result("ok", "done", &[], None, true, 1, &[], &[], None);

        assert!(output["failure_category"].is_null());
    }

    #[test]
    fn test_normalize_non_apply_failure_category_fallback() {
        let mut output = json!({
            "status": "error",
            "code": "",
            "explanation": "failed",
            "files_modified": []
        });

        normalize_non_apply_output(&mut output);

        assert_eq!(output["failure_category"], json!("unknown"));
    }

    #[test]
    fn test_verify_timeout_category() {
        assert_eq!(
            verify_failure_category("verify command timed out after 120000ms"),
            "timeout"
        );
        assert_eq!(verify_failure_category("tests failed"), "verify");
    }

    #[test]
    fn test_result_failure_category_approved_taxonomy() {
        for category in [
            "format",
            "schema",
            "preflight",
            "verify",
            "timeout",
            "network",
            "model",
            "unknown",
        ] {
            let value = json!({ "failure_category": category });
            assert_eq!(result_failure_category(&value), Some(category));
        }
    }
}
