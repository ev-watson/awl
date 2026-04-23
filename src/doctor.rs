use std::path::PathBuf;
use std::time::Duration;

use crate::defaults;
use crate::session;

/// Run health checks and print results. Returns Ok if all critical checks pass.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut passed = 0u32;
    let mut failed = 0u32;
    let base_url = defaults::configured_ollama_base_url();
    let api_root = defaults::ollama_api_root(&base_url);
    let tags_url = defaults::ollama_tags_url(&base_url);
    let runtime = tokio::runtime::Runtime::new()?;
    let tags_body = runtime.block_on(fetch_ollama_tags(&tags_url));

    // 1. Ollama reachability
    print!("  Ollama API ({api_root}) ... ");
    match tags_body.as_ref() {
        Ok(_) => {
            println!("ok");
            passed += 1;
        }
        Err(e) => {
            println!("FAIL: {e}");
            failed += 1;
        }
    }

    // 2. Model availability
    print!("  Default model ({}) ... ", defaults::DEFAULT_AGENT_MODEL);
    match tags_body
        .as_deref()
        .map_err(ToOwned::to_owned)
        .and_then(|body| check_model_available(body, defaults::DEFAULT_AGENT_MODEL))
    {
        Ok(()) => {
            println!("ok");
            passed += 1;
        }
        Err(e) => {
            println!("FAIL: {e}");
            eprintln!(
                "    hint: run `ollama pull {}`",
                defaults::DEFAULT_AGENT_MODEL
            );
            failed += 1;
        }
    }

    // 3. Sessions directory
    print!("  Sessions directory ... ");
    match check_sessions_dir() {
        Ok(info) => {
            println!("ok ({info})");
            passed += 1;
        }
        Err(e) => {
            println!("FAIL: {e}");
            failed += 1;
        }
    }

    // 4. Workspace root
    print!("  Workspace root (cwd) ... ");
    match crate::safety::workspace_root() {
        Ok(root) => {
            println!("ok ({})", root.display());
            passed += 1;
        }
        Err(e) => {
            println!("FAIL: {e}");
            failed += 1;
        }
    }

    // 5. MCP config (optional)
    print!("  MCP config ({}) ... ", defaults::DEFAULT_MCP_CONFIG_FILE);
    let mcp_path = PathBuf::from(defaults::DEFAULT_MCP_CONFIG_FILE);
    if mcp_path.exists() {
        match crate::mcp_client::load_mcp_config(&mcp_path) {
            Ok(configs) => {
                println!("ok ({} server(s))", configs.len());
                passed += 1;
            }
            Err(e) => {
                println!("FAIL: {e}");
                failed += 1;
            }
        }
    } else {
        println!("skipped (file not found)");
    }

    println!();
    if failed > 0 {
        println!("  {passed} passed, {failed} failed");
        Err(format!("{failed} health check(s) failed").into())
    } else {
        println!("  all {passed} checks passed");
        Ok(())
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
    let sessions = session::list_sessions().map_err(|e| e.to_string())?;
    Ok(format!("{} session(s)", sessions.len()))
}
