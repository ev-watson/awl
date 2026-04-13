use std::io::{self, Read};
use std::process;

mod agent;
mod dispatch;
mod hashline;
mod mcp_client;
mod phases;
mod plan;
mod repomap;
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
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        "--version" | "-V" => {
            println!("claw {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(format!("unknown subcommand: {other}\n\nRun `claw --help` for usage.").into()),
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
        "claw — local agentic coding dispatch CLI

USAGE:
    claw dispatch --level {{2,3}} < task.json
    claw hashline read <file>
    claw hashline apply <file> < edits
    claw repomap [--path .] [--budget 4096] [--focus file.rs]
    claw plan [--level 2] < task.json
    claw agent --task \"description\" [--offline] [--model name] [--mcp-config file] [--resume id]
    claw --version
    claw --help

SUBCOMMANDS:
    dispatch    Send a task to a local Ollama model
                --level 2  Qwen2.5-Coder 7B (implementation)
                --level 3  Qwen2.5-Coder 3B (verification)
    hashline    Content-hashed line references for stable edits
                read <file>   Display file with LINE:HASH|content tags
                apply <file>  Apply edit operations from stdin
    repomap     PageRank-ranked code map for codebase context
                --path .      Root directory to scan (default: cwd)
                --budget 4096 Max output tokens (default: 4096)
                --focus f.rs  Comma-separated files to prioritize
    plan        Ask Level 2/3 to decompose a task into an implementation plan
                --level 2     Model to use for planning (default: 2)
    agent       Offline tool-use agent loop with phase discipline
                --task \"desc\"        Task description
                --offline             Print offline-mode warning
                --model <name>        Override model (default: qwen2.5-coder:14b)
                --mcp-config <path>   MCP server config file
                --resume <session-id> Resume a session log

STDIN FORMAT (dispatch/plan, JSON):
    {{
        \"task\": \"description of what to do\",
        \"context\": \"optional relevant code or context\",
        \"constraints\": [\"optional\", \"constraint\", \"list\"]
    }}"
    );
}
