use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config;
use crate::defaults;
use crate::session;

#[derive(Default)]
struct CheckCounts {
    passed: u32,
    failed: u32,
}

/// Run health checks and print results. Returns Ok if all critical checks pass.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut counts = CheckCounts::default();
    let config_path = config::config_path()?;
    let base_url = defaults::configured_ollama_base_url();
    let api_root = defaults::ollama_api_root(&base_url);
    let tags_url = defaults::ollama_tags_url(&base_url);
    let runtime = tokio::runtime::Runtime::new()?;
    let tags_body = runtime.block_on(fetch_ollama_tags(&tags_url));

    check_config_file(&config_path, &mut counts);
    check_ollama_api(&api_root, &tags_body, &mut counts);
    check_configured_models(&tags_body, &mut counts)?;
    check_sessions_directory(&mut counts);
    check_workspace_root(&mut counts);
    check_optional_mcp_config(&mut counts);

    println!();
    if counts.failed > 0 {
        println!("  {} passed, {} failed", counts.passed, counts.failed);
        Err(format!("{} health check(s) failed", counts.failed).into())
    } else {
        println!("  all {} checks passed", counts.passed);
        Ok(())
    }
}

fn check_config_file(config_path: &Path, counts: &mut CheckCounts) {
    print!("  Config file ({}) ... ", config_path.display());
    if config_path.exists() {
        match config::load() {
            Ok(_) => {
                println!("ok");
                counts.passed += 1;
            }
            Err(error) => {
                println!("FAIL: {error}");
                counts.failed += 1;
            }
        }
    } else {
        println!("skipped (using built-in defaults)");
    }
}

fn check_ollama_api(api_root: &str, tags_body: &Result<String, String>, counts: &mut CheckCounts) {
    print!("  Ollama API ({api_root}) ... ");
    match tags_body.as_ref() {
        Ok(_) => {
            println!("ok");
            counts.passed += 1;
        }
        Err(e) => {
            println!("FAIL: {e}");
            counts.failed += 1;
        }
    }
}

fn check_configured_models(
    tags_body: &Result<String, String>,
    counts: &mut CheckCounts,
) -> Result<(), Box<dyn std::error::Error>> {
    let configured_models = [
        ("Agent model", defaults::configured_agent_model()),
        (
            "Implementation model",
            defaults::configured_model_for_level(2)?,
        ),
        (
            "Verification model",
            defaults::configured_model_for_level(3)?,
        ),
    ];
    for (label, model) in configured_models {
        print!("  {label} ({model}) ... ");
        match tags_body
            .as_deref()
            .map_err(ToOwned::to_owned)
            .and_then(|body| check_model_available(body, &model))
        {
            Ok(()) => {
                println!("ok");
                counts.passed += 1;
            }
            Err(error) => {
                println!("FAIL: {error}");
                eprintln!("    hint: run `ollama pull {model}`");
                counts.failed += 1;
            }
        }
    }
    Ok(())
}

fn check_sessions_directory(counts: &mut CheckCounts) {
    print!("  Sessions directory ... ");
    match check_sessions_dir() {
        Ok(info) => {
            println!("ok ({info})");
            counts.passed += 1;
        }
        Err(e) => {
            println!("FAIL: {e}");
            counts.failed += 1;
        }
    }
}

fn check_workspace_root(counts: &mut CheckCounts) {
    print!("  Workspace root (cwd) ... ");
    match crate::safety::workspace_root() {
        Ok(root) => {
            println!("ok ({})", root.display());
            counts.passed += 1;
        }
        Err(e) => {
            println!("FAIL: {e}");
            counts.failed += 1;
        }
    }
}

fn check_optional_mcp_config(counts: &mut CheckCounts) {
    let mcp_path = defaults::configured_mcp_config_path()
        .unwrap_or_else(|| PathBuf::from(defaults::DEFAULT_MCP_CONFIG_FILE));
    print!("  MCP config ({}) ... ", mcp_path.display());
    if mcp_path.exists() {
        match crate::mcp_client::load_mcp_config(&mcp_path) {
            Ok(configs) => {
                println!("ok ({} server(s))", configs.len());
                counts.passed += 1;
            }
            Err(e) => {
                println!("FAIL: {e}");
                counts.failed += 1;
            }
        }
    } else {
        println!("skipped (file not found)");
    }
}

async fn fetch_ollama_tags(tags_url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;
    let response = client
        .get(tags_url)
        .send()
        .await
        .map_err(|e| format!("Ollama not responding at {tags_url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Ollama returned HTTP {} from {tags_url}",
            response.status()
        ));
    }
    response
        .text()
        .await
        .map_err(|e| format!("failed to read Ollama response from {tags_url}: {e}"))
}

fn check_model_available(tags_body: &str, model: &str) -> Result<(), String> {
    if tags_body.contains(model) {
        Ok(())
    } else {
        Err(format!("model {model} not found in Ollama"))
    }
}

fn check_sessions_dir() -> Result<String, String> {
    let dir = config::configured_sessions_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let sessions = session::list_sessions().map_err(|e| e.to_string())?;
    Ok(format!(
        "{} session(s) at {}",
        sessions.len(),
        dir.display()
    ))
}
