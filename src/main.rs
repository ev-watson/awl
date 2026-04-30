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
            let options = parse_dispatch_options(&args[1..])?;
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            dispatch::run(&options, &input)
        }
        "dispatches" => dispatch::run_logs(&args[1..]),
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

fn parse_dispatch_options(
    args: &[String],
) -> Result<dispatch::DispatchOptions, Box<dyn std::error::Error>> {
    let mut level: Option<u8> = None;
    let mut apply = false;
    let mut verify_command: Option<String> = None;
    let mut target_path: Option<String> = None;
    let mut max_attempts: Option<usize> = None;
    let mut max_return_chars: Option<usize> = None;
    let mut auto_repomap = false;
    let mut repomap_focus: Vec<String> = Vec::new();
    let mut repomap_budget: Option<usize> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--level" => {
                i += 1;
                let parsed = args
                    .get(i)
                    .ok_or("--level requires a value")?
                    .parse()
                    .map_err(|_| "--level must be 2 or 3")?;
                level = Some(parsed);
            }
            "--apply" | "--write" => {
                apply = true;
            }
            "--verify" => {
                i += 1;
                verify_command = Some(args.get(i).cloned().ok_or("--verify requires a value")?);
            }
            "--target-path" | "--target-file" => {
                i += 1;
                target_path = Some(
                    args.get(i)
                        .cloned()
                        .ok_or("--target-path requires a value")?,
                );
            }
            "--max-attempts" => {
                i += 1;
                max_attempts = Some(
                    args.get(i)
                        .ok_or("--max-attempts requires a value")?
                        .parse()
                        .map_err(|_| "--max-attempts must be a positive integer")?,
                );
            }
            "--max-return-chars" => {
                i += 1;
                max_return_chars = Some(
                    args.get(i)
                        .ok_or("--max-return-chars requires a value")?
                        .parse()
                        .map_err(|_| "--max-return-chars must be a positive integer")?,
                );
            }
            "--auto-repomap" => {
                auto_repomap = true;
            }
            "--repomap-focus" => {
                i += 1;
                let raw = args.get(i).ok_or("--repomap-focus requires a value")?;
                repomap_focus.extend(
                    raw.split(',')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string),
                );
            }
            "--repomap-budget" => {
                i += 1;
                repomap_budget = Some(
                    args.get(i)
                        .ok_or("--repomap-budget requires a value")?
                        .parse()
                        .map_err(|_| "--repomap-budget must be a positive integer")?,
                );
            }
            other => {
                return Err(format!(
                    "unknown dispatch flag: {other}\n\nRun `awl --help` for usage."
                )
                .into());
            }
        }
        i += 1;
    }

    let level = level.ok_or("dispatch requires --level {2,3}")?;
    if level != 2 && level != 3 {
        return Err("--level must be 2 or 3".into());
    }

    let mut options = dispatch::DispatchOptions::new(level);
    options.apply = apply;
    options.verify_command = verify_command;
    options.target_path = target_path;
    options.max_attempts = max_attempts;
    options.max_return_chars = max_return_chars;
    options.auto_repomap = auto_repomap;
    options.repomap_focus = repomap_focus;
    options.repomap_budget = repomap_budget;
    Ok(options)
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
                --apply    Write target_path/target_files[0] locally
                --verify \"cmd\" Run a check after apply, rollback on failure
                --target-path <file> Override JSON target_path
                --max-attempts <n> Local apply/verify attempts, capped at 5
                --auto-repomap Inject a small local repo map into the worker prompt
                --repomap-focus <files> Comma-separated focus files
                --repomap-budget <n> Token budget for auto repomap
                Reads JSON task from stdin.

    hashline    Content-hashed line references for stable edits
                read <file>   Display file with LINE:HASH|content tags
                apply <file>  Apply edit operations from stdin

    dispatches  Inspect local dispatch attempt logs
                --list         List dispatch logs (default)
                --show <id>    Print a dispatch JSONL log
                --tail <id>    Print the last 20 log events
                --prune <days> Delete dispatch logs older than N days

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
                --max-iterations <n>  Stop after N agent loop iterations
                --max-text-without-tool <n> Stop after N text-only turns
                --max-wall-seconds <n> Stop after N wall-clock seconds

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
        \"constraints\": [\"optional\", \"constraint\", \"list\"],
        \"target_path\": \"optional/file/to/write\",
        \"context_paths\": [\"optional files Awl should read locally\"],
        \"auto_repomap\": false,
        \"repomap_focus\": [\"optional focus files\"],
        \"verify_command\": \"optional command for --apply mode\"
    }}",
        env!("CARGO_PKG_VERSION"),
        default_budget = crate::defaults::DEFAULT_REPOMAP_BUDGET,
        default_model = crate::defaults::configured_agent_model(),
    );
}
