use std::path::PathBuf;

use crate::config;

pub const DEFAULT_AGENT_MODEL: &str = "qwen2.5-coder:14b";
pub const DEFAULT_IMPLEMENTATION_MODEL: &str = "qwen2.5-coder:7b-instruct-q4_K_M";
pub const DEFAULT_VERIFICATION_MODEL: &str = "qwen2.5-coder:3b-instruct-q4_K_M";
pub const LITE_AGENT_MODEL: &str = DEFAULT_IMPLEMENTATION_MODEL;
pub const LITE_IMPLEMENTATION_MODEL: &str = DEFAULT_VERIFICATION_MODEL;
pub const LITE_VERIFICATION_MODEL: &str = DEFAULT_VERIFICATION_MODEL;
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434/v1";
pub const DEFAULT_REPOMAP_BUDGET: usize = 4096;
pub const DEFAULT_MCP_CONFIG_FILE: &str = "mcp-awl-server.json";
pub const AGENT_MODEL_ENV: &str = "AWL_AGENT_MODEL";
pub const IMPLEMENTATION_MODEL_ENV: &str = "AWL_IMPLEMENTATION_MODEL";
pub const VERIFICATION_MODEL_ENV: &str = "AWL_VERIFICATION_MODEL";
pub const OLLAMA_BASE_URL_ENV: &str = "OLLAMA_BASE_URL";
pub const OLLAMA_HOST_ENV: &str = "OLLAMA_HOST";
pub const ENABLE_MCP_AGENT_ENV: &str = "AWL_ENABLE_MCP_AGENT";

pub fn configured_ollama_base_url() -> String {
    let configured = config::load().ok().and_then(|loaded| loaded.base_url);
    let env_base_url = std::env::var(OLLAMA_BASE_URL_ENV).ok();
    let env_host = std::env::var(OLLAMA_HOST_ENV).ok();
    configured_ollama_base_url_from(
        env_base_url
            .as_deref()
            .or(env_host.as_deref())
            .or(configured.as_deref()),
    )
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

pub fn max_tokens_for_level(level: u8) -> Result<u32, String> {
    match level {
        2 => Ok(8192),
        3 => Ok(4096),
        _ => Err(format!("invalid level {level}: expected 2 or 3")),
    }
}

pub fn configured_agent_model() -> String {
    configured_string(
        AGENT_MODEL_ENV,
        |loaded| loaded.agent_model,
        DEFAULT_AGENT_MODEL,
    )
}

pub fn configured_model_for_level(level: u8) -> Result<String, String> {
    match level {
        2 => Ok(configured_string(
            IMPLEMENTATION_MODEL_ENV,
            |loaded| loaded.implementation_model,
            DEFAULT_IMPLEMENTATION_MODEL,
        )),
        3 => Ok(configured_string(
            VERIFICATION_MODEL_ENV,
            |loaded| loaded.verification_model,
            DEFAULT_VERIFICATION_MODEL,
        )),
        _ => Err(format!("invalid level {level}: expected 2 or 3")),
    }
}

pub fn configured_mcp_config_path() -> Option<PathBuf> {
    config::configured_mcp_config_path().or_else(|| {
        let candidate = PathBuf::from(DEFAULT_MCP_CONFIG_FILE);
        candidate.exists().then_some(candidate)
    })
}

pub fn mcp_agent_enabled() -> bool {
    std::env::var(ENABLE_MCP_AGENT_ENV)
        .ok()
        .is_some_and(|value| matches_enabled(&value))
}

fn normalize_ollama_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return DEFAULT_OLLAMA_BASE_URL.to_string();
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };
    let without_trailing_slash = with_scheme.trim_end_matches('/');
    if without_trailing_slash.ends_with("/v1") {
        without_trailing_slash.to_string()
    } else {
        format!("{without_trailing_slash}/v1")
    }
}

fn configured_string(
    env_key: &str,
    config_value: impl Fn(config::UserConfig) -> Option<String>,
    fallback: &str,
) -> String {
    std::env::var(env_key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| config::load().ok().and_then(config_value))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn matches_enabled(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
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
    fn base_url_normalization_accepts_ollama_host_style_values() {
        assert_eq!(
            configured_ollama_base_url_from(Some("localhost:11434")),
            "http://localhost:11434/v1"
        );
        assert_eq!(
            configured_ollama_base_url_from(Some("127.0.0.1:11434/")),
            "http://127.0.0.1:11434/v1"
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

    #[test]
    fn mcp_agent_env_parsing_accepts_common_enabled_values() {
        assert!(matches_enabled("1"));
        assert!(matches_enabled("true"));
        assert!(matches_enabled("YES"));
        assert!(matches_enabled("on"));
        assert!(!matches_enabled("0"));
        assert!(!matches_enabled("false"));
        assert!(!matches_enabled(""));
    }
}
