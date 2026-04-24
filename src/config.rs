use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CONFIG_PATH_ENV: &str = "AWL_CONFIG_PATH";
pub const CONFIG_DIR_ENV: &str = "AWL_CONFIG_DIR";
pub const SESSIONS_DIR_ENV: &str = "AWL_SESSIONS_DIR";
pub const MCP_CONFIG_ENV: &str = "AWL_MCP_CONFIG";

const APP_DIR_NAME: &str = "awl";
const CONFIG_FILE_NAME: &str = "config.json";
const SESSIONS_DIR_NAME: &str = "sessions";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sessions_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_config: Option<PathBuf>,
}

pub fn config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os(CONFIG_PATH_ENV) {
        return Ok(PathBuf::from(path));
    }
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

pub fn config_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(dir) = std::env::var_os(CONFIG_DIR_ENV) {
        return Ok(PathBuf::from(dir));
    }

    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(dir).join(APP_DIR_NAME));
    }

    if let Some(dir) = std::env::var_os("APPDATA") {
        return Ok(PathBuf::from(dir).join(APP_DIR_NAME));
    }

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or("failed to resolve config directory: HOME/USERPROFILE is not set")?;
    Ok(PathBuf::from(home).join(".config").join(APP_DIR_NAME))
}

pub fn default_sessions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(config_dir()?.join(SESSIONS_DIR_NAME))
}

pub fn configured_sessions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(dir) = std::env::var_os(SESSIONS_DIR_ENV) {
        return Ok(PathBuf::from(dir));
    }

    let config = load()?;
    if let Some(dir) = config.sessions_dir {
        return Ok(dir);
    }

    default_sessions_dir()
}

pub fn configured_mcp_config_path() -> Option<PathBuf> {
    std::env::var_os(MCP_CONFIG_ENV)
        .map(PathBuf::from)
        .or_else(|| load().ok().and_then(|config| config.mcp_config))
}

pub fn load() -> Result<UserConfig, Box<dyn std::error::Error>> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(UserConfig::default());
    }

    let raw = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save(config: &UserConfig) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let encoded = serde_json::to_string_pretty(config)?;
    fs::write(&path, format!("{encoded}\n"))?;
    Ok(path)
}

pub fn path_display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn run_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args.first().map_or("show", String::as_str) {
        "--help" | "-h" | "help" => {
            println!("Usage:\n  awl config show\n  awl config path");
            Ok(())
        }
        "path" => {
            println!("{}", config_path()?.display());
            Ok(())
        }
        "show" => {
            let path = config_path()?;
            let config = load()?;
            println!("Config path: {}", path.display());
            println!("{}", serde_json::to_string_pretty(&config)?);
            Ok(())
        }
        other => Err(format!(
            "unknown config subcommand: {other}\n\nUsage:\n  awl config show\n  awl config path"
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_prefers_xdg_home() {
        let path = app_dir_from(
            None,
            Some(Path::new("/tmp/config-home")),
            Some(Path::new("/tmp/appdata")),
            Some(Path::new("/tmp/home")),
        );
        assert_eq!(path, PathBuf::from("/tmp/config-home/awl"));
    }

    #[test]
    fn config_dir_falls_back_to_home_config() {
        let path = app_dir_from(None, None, None, Some(Path::new("/tmp/home")));
        assert_eq!(path, PathBuf::from("/tmp/home/.config/awl"));
    }

    #[test]
    fn config_dir_uses_custom_dir_override() {
        let path = app_dir_from(
            Some(Path::new("/tmp/awl-config")),
            Some(Path::new("/tmp/config-home")),
            None,
            Some(Path::new("/tmp/home")),
        );
        assert_eq!(path, PathBuf::from("/tmp/awl-config"));
    }

    fn app_dir_from(
        custom: Option<&Path>,
        xdg: Option<&Path>,
        appdata: Option<&Path>,
        home: Option<&Path>,
    ) -> PathBuf {
        if let Some(dir) = custom {
            return dir.to_path_buf();
        }
        if let Some(dir) = xdg {
            return dir.join(APP_DIR_NAME);
        }
        if let Some(dir) = appdata {
            return dir.join(APP_DIR_NAME);
        }
        home.expect("test home").join(".config").join(APP_DIR_NAME)
    }
}
