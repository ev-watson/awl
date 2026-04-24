use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::config::{self, UserConfig};
use crate::defaults;

enum Profile {
    Default,
    Lite,
}

#[derive(Default)]
struct InitOptions {
    profile: Option<Profile>,
    base_url: Option<String>,
    agent_model: Option<String>,
    implementation_model: Option<String>,
    verification_model: Option<String>,
    sessions_dir: Option<PathBuf>,
    mcp_config: Option<PathBuf>,
    no_check: bool,
}

impl Profile {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "default" => Ok(Self::Default),
            "lite" => Ok(Self::Lite),
            other => Err(format!(
                "unknown init profile `{other}`. Expected `default` or `lite`"
            )),
        }
    }

    fn apply(&self, config: &mut UserConfig) {
        match self {
            Self::Default => {
                config.agent_model = Some(defaults::DEFAULT_AGENT_MODEL.to_string());
                config.implementation_model =
                    Some(defaults::DEFAULT_IMPLEMENTATION_MODEL.to_string());
                config.verification_model = Some(defaults::DEFAULT_VERIFICATION_MODEL.to_string());
            }
            Self::Lite => {
                config.agent_model = Some(defaults::LITE_AGENT_MODEL.to_string());
                config.implementation_model = Some(defaults::LITE_IMPLEMENTATION_MODEL.to_string());
                config.verification_model = Some(defaults::LITE_VERIFICATION_MODEL.to_string());
            }
        }
    }
}

pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h" | "help"))
    {
        print_usage();
        return Ok(());
    }

    let options = parse_args(args)?;
    let mut user_config = config::load()?;
    apply_options(&mut user_config, &options);
    fill_missing_defaults(&mut user_config)?;
    let path = config::save(&user_config)?;
    print_summary(&path)?;

    if options.no_check {
        println!("\nSkipped health checks (`--no-check`). Run `awl doctor` when ready.");
        return Ok(());
    }

    println!("\nRunning health checks...\n");
    crate::doctor::run().map_err(|error| {
        format!("configuration saved, but health checks did not pass: {error}").into()
    })
}

fn parse_args(args: &[String]) -> Result<InitOptions, Box<dyn std::error::Error>> {
    let mut options = InitOptions::default();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--profile" => {
                i += 1;
                let value = args.get(i).ok_or("--profile requires a value")?;
                options.profile = Some(Profile::parse(value)?);
            }
            "--base-url" => {
                i += 1;
                options.base_url = Some(args.get(i).cloned().ok_or("--base-url requires a value")?);
            }
            "--agent-model" => {
                i += 1;
                options.agent_model = Some(
                    args.get(i)
                        .cloned()
                        .ok_or("--agent-model requires a value")?,
                );
            }
            "--implementation-model" => {
                i += 1;
                options.implementation_model = Some(
                    args.get(i)
                        .cloned()
                        .ok_or("--implementation-model requires a value")?,
                );
            }
            "--verification-model" => {
                i += 1;
                options.verification_model = Some(
                    args.get(i)
                        .cloned()
                        .ok_or("--verification-model requires a value")?,
                );
            }
            "--sessions-dir" => {
                i += 1;
                options.sessions_dir = Some(PathBuf::from(
                    args.get(i).ok_or("--sessions-dir requires a value")?,
                ));
            }
            "--mcp-config" => {
                i += 1;
                options.mcp_config = Some(PathBuf::from(
                    args.get(i).ok_or("--mcp-config requires a value")?,
                ));
            }
            "--no-check" => options.no_check = true,
            other => {
                return Err(
                    format!("unknown init flag: {other}\n\nRun `awl help` for usage.").into(),
                );
            }
        }
        i += 1;
    }

    Ok(options)
}

fn apply_options(config: &mut UserConfig, options: &InitOptions) {
    if let Some(profile) = &options.profile {
        profile.apply(config);
    }
    if let Some(base_url) = &options.base_url {
        config.base_url = Some(base_url.clone());
    }
    if let Some(agent_model) = &options.agent_model {
        config.agent_model = Some(agent_model.clone());
    }
    if let Some(implementation_model) = &options.implementation_model {
        config.implementation_model = Some(implementation_model.clone());
    }
    if let Some(verification_model) = &options.verification_model {
        config.verification_model = Some(verification_model.clone());
    }
    if let Some(sessions_dir) = &options.sessions_dir {
        config.sessions_dir = Some(sessions_dir.clone());
    }
    if let Some(mcp_config) = &options.mcp_config {
        config.mcp_config = Some(mcp_config.clone());
    }
}

fn fill_missing_defaults(config: &mut UserConfig) -> Result<(), Box<dyn std::error::Error>> {
    if config.base_url.is_none() {
        config.base_url = Some(defaults::configured_ollama_base_url());
    }
    if config.agent_model.is_none() {
        config.agent_model = Some(defaults::configured_agent_model());
    }
    if config.implementation_model.is_none() {
        config.implementation_model = Some(defaults::configured_model_for_level(2)?);
    }
    if config.verification_model.is_none() {
        config.verification_model = Some(defaults::configured_model_for_level(3)?);
    }
    if config.sessions_dir.is_none() {
        config.sessions_dir = Some(config::default_sessions_dir()?);
    }
    if config.mcp_config.is_none() {
        config.mcp_config = defaults::configured_mcp_config_path();
    }
    Ok(())
}

fn print_summary(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Initialized Awl config at {}", path.display());
    println!("  base_url: {}", defaults::configured_ollama_base_url());
    println!("  agent_model: {}", defaults::configured_agent_model());
    println!(
        "  implementation_model: {}",
        defaults::configured_model_for_level(2)?
    );
    println!(
        "  verification_model: {}",
        defaults::configured_model_for_level(3)?
    );
    println!(
        "  sessions_dir: {}",
        config::path_display(&config::configured_sessions_dir()?)
    );
    if let Some(path) = defaults::configured_mcp_config_path() {
        println!("  mcp_config: {}", path.display());
    }
    println!("\nPull the configured models if they are not already installed:");
    for model in configured_models()? {
        println!("  ollama pull {model}");
    }
    Ok(())
}

fn configured_models() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut models = Vec::new();
    let mut seen = HashSet::new();
    for model in [
        defaults::configured_agent_model(),
        defaults::configured_model_for_level(2)?,
        defaults::configured_model_for_level(3)?,
    ] {
        if seen.insert(model.clone()) {
            models.push(model);
        }
    }
    Ok(models)
}

fn print_usage() {
    println!(
        "Usage:
  awl init [options]

Options:
  --profile default|lite
  --base-url <url>
  --agent-model <name>
  --implementation-model <name>
  --verification-model <name>
  --sessions-dir <path>
  --mcp-config <path>
  --no-check

Examples:
  awl init --profile lite
  awl init --base-url http://192.168.1.10:11434 --profile default
  awl init --agent-model qwen2.5-coder:7b-instruct-q4_K_M --no-check"
    );
}
