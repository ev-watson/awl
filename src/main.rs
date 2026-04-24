use std::io::{self, Read};
use std::process;

mod agent;
mod config;
mod defaults;
mod dispatch;
mod doctor;
mod hashline;
mod init;
mod llm_io;
mod mcp_client;
mod mcp_server;
mod phases;
mod plan;
mod repomap;
mod safety;
mod session;
mod tools;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "dispatch" => {
            let level = parse_dispatch_level(&args[1..])?;
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            dispatch::run(level, &input)
        }
        "hashline" => hashline::run(&args[1..]),
        "repomap" => repomap::run(&args[1..]),
        "plan" => plan::run(&args[1..]),
        "agent" => agent::run_agent_cli(&args[1..]),
        "init" => init::run(&args[1..]),
        "config" => config::run_cli(&args[1..]),
        "serve" => mcp_server::run_server(),
        "doctor" => {
            println!("awl doctor — health checks\n");
            doctor::run()
        }
        "sessions" => run_sessions(&args[1..]),
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        "--version" | "-V" => {
            println!("awl {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(format!("unknown subcommand: {other}\n\nRun `awl --help` for usage.").into()),
    }
}

fn run_sessions(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args[0] == "--list" {
        let sessions = session::list_sessions()?;
        if sessions.is_empty() {
            println!("no sessions found");
        } else {
            println!("{:<45} {:>8}  MODIFIED", "SESSION ID", "SIZE");
            for s in &sessions {
                let age = s.modified.elapsed().map_or_else(
                    |_| "unknown".to_string(),
                    |d| format!("{}d ago", d.as_secs() / 86400),
                );
                let size = if s.size_bytes > 1024 {
                    format!("{}K", s.size_bytes / 1024)
                } else {
                    format!("{}B", s.size_bytes)
                };
                println!("{:<45} {:>8}  {}", s.id, size, age);
            }
            println!("\n{} session(s)", sessions.len());
        }
        Ok(())
    } else if args[0] == "--prune" {
        let days: u64 = args
            .get(1)
            .ok_or("--prune requires a number of days (e.g., `awl sessions --prune 30`)")?
            .parse()
            .map_err(|_| "--prune value must be a positive integer (days)")?;
        let deleted = session::prune_sessions(days)?;
        println!("deleted {deleted} session(s) older than {days} days");
        Ok(())
    } else {
        Err(format!(
            "unknown sessions flag: {}\n\nUsage:\n  awl sessions [--list]\n  awl sessions --prune <days>",
            args[0]
        )
        .into())
    }
}

fn parse_dispatch_level(args: &[String]) -> Result<u8, Box<dyn std::error::Error>> {
    let pos = args
        .iter()
        .position(|a| a == "--level")
        .ok_or("dispatch requires --level {2,3}")?;
    let level: u8 = args
        .get(pos + 1)
        .ok_or("--level requires a value")?
        .parse()
        .map_err(|_| "--level must be 2 or 3")?;
    if level != 2 && level != 3 {
        return Err("--level must be 2 or 3".into());
    }
    Ok(level)
}

fn print_usage() {
    println!(
        "awl {} — local agentic coding dispatch CLI

USAGE:
    awl <subcommand> [options]

SUBCOMMANDS:
    dispatch    Send a task to a local Ollama model
                --level 2  Qwen2.5-Coder 7B (implementation)
                --level 3  Qwen2.5-Coder 3B (verification)
                Reads JSON task from stdin.

    hashline    Content-hashed line references for stable edits
                read <file>   Display file with LINE:HASH|content tags
                apply <file>  Apply edit operations from stdin

    repomap     PageRank-ranked code map for codebase context
                --path .      Root directory to scan (default: cwd)
                --budget {default_budget} Max output tokens (default: {default_budget})
                --focus f.rs  Comma-separated files to prioritize

    init        Create or update the user config file
                --profile default|lite
                --base-url <url>
                --agent-model <name>
                --implementation-model <name>
                --verification-model <name>
                --sessions-dir <path>
                --mcp-config <path>
                --no-check          Skip post-init health checks

    config      Inspect saved configuration
                show                Print current config file contents (default)
                path                Print config file path

    plan        Ask Level 2/3 to decompose a task into a plan
                --level 2     Model to use for planning (default: 2)
                Reads JSON task from stdin.

    agent       Tool-use agent loop with phase discipline
                --task \"desc\"        Task description (or pipe via stdin)
                --persona \"...\"      Domain expertise framing
                --goal \"...\"         Explicit research goal
                --idea \"...\"         User hypothesis (repeatable)
                --model <name>        Override model (default: {default_model})
                --mcp-config <path>   MCP server config file
                --resume <session-id> Resume a saved session

    serve       Start MCP server on stdio (for Claude Code integration)

    doctor      Run health checks (Ollama, models, sessions, workspace)

    sessions    Manage session logs
                --list                List all sessions (default)
                --prune <days>        Delete sessions older than N days

    --version   Print version
    --help      Print this help

STDIN FORMAT (dispatch/plan):
    {{
        \"task\": \"description of what to do\",
        \"context\": \"optional relevant code or context\",
        \"constraints\": [\"optional\", \"constraint\", \"list\"]
    }}",
        env!("CARGO_PKG_VERSION"),
        default_budget = crate::defaults::DEFAULT_REPOMAP_BUDGET,
        default_model = crate::defaults::configured_agent_model(),
    );
}
