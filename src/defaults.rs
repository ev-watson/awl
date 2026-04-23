pub const DEFAULT_AGENT_MODEL: &str = "qwen2.5-coder:14b";
pub const DEFAULT_IMPLEMENTATION_MODEL: &str = "qwen2.5-coder:7b-instruct-q4_K_M";
pub const DEFAULT_VERIFICATION_MODEL: &str = "qwen2.5-coder:3b-instruct-q4_K_M";
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434/v1";
pub const DEFAULT_REPOMAP_BUDGET: usize = 4096;
pub const DEFAULT_MCP_CONFIG_FILE: &str = "mcp-awl-server.json";

pub fn configured_ollama_base_url() -> String {
    configured_ollama_base_url_from(std::env::var("OLLAMA_BASE_URL").ok().as_deref())
}

pub fn configured_ollama_base_url_from(raw: Option<&str>) -> String {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(
            || DEFAULT_OLLAMA_BASE_URL.to_string(),
            normalize_ollama_base_url,
        )
}

pub fn ollama_api_root(base_url: &str) -> String {
    let normalized = normalize_ollama_base_url(base_url.trim());
    normalized
        .strip_suffix("/v1")
        .unwrap_or(&normalized)
        .to_string()
}

pub fn ollama_chat_completions_url(base_url: &str) -> String {
    format!("{}/chat/completions", normalize_ollama_base_url(base_url))
}

pub fn ollama_tags_url(base_url: &str) -> String {
    format!("{}/api/tags", ollama_api_root(base_url))
}

pub fn model_for_level(level: u8) -> Result<&'static str, String> {
    match level {
        2 => Ok(DEFAULT_IMPLEMENTATION_MODEL),
        3 => Ok(DEFAULT_VERIFICATION_MODEL),
        _ => Err(format!("invalid level {level}: expected 2 or 3")),
    }
}

pub fn max_tokens_for_level(level: u8) -> Result<u32, String> {
    match level {
        2 => Ok(8192),
        3 => Ok(4096),
        _ => Err(format!("invalid level {level}: expected 2 or 3")),
    }
}

fn normalize_ollama_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return DEFAULT_OLLAMA_BASE_URL.to_string();
    }
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_defaults_when_env_is_missing_or_blank() {
        assert_eq!(
            configured_ollama_base_url_from(None),
            DEFAULT_OLLAMA_BASE_URL
        );
        assert_eq!(
            configured_ollama_base_url_from(Some("  ")),
            DEFAULT_OLLAMA_BASE_URL
        );
    }

    #[test]
    fn base_url_normalization_appends_openai_compat_suffix() {
        assert_eq!(
            configured_ollama_base_url_from(Some("http://localhost:11434")),
            "http://localhost:11434/v1"
        );
        assert_eq!(
            configured_ollama_base_url_from(Some("http://localhost:11434/v1/")),
            "http://localhost:11434/v1"
        );
    }

    #[test]
    fn tags_url_is_derived_from_api_root() {
        assert_eq!(
            ollama_tags_url("http://localhost:11434/v1"),
            "http://localhost:11434/api/tags"
        );
        assert_eq!(
            ollama_tags_url("http://localhost:11434"),
            "http://localhost:11434/api/tags"
        );
    }
}
