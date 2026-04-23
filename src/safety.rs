use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

const ALLOWED_SHELL_COMMANDS: &[&str] = &[
    "cargo", "git", "rg", "grep", "find", "ls", "cat", "sed", "awk", "head", "tail", "wc", "pwd",
    "echo", "printf", "cut", "sort", "uniq", "tr", "basename", "dirname", "stat", "file",
    "python3", "python", "node", "make", "mkdir", "touch", "cp", "mv", "rm", "diff",
];

const FORBIDDEN_SHELL_FRAGMENTS: &[&str] = &["\n", ";", "`", "$("];
const ALLOWED_CARGO_SUBCOMMANDS: &[&str] = &[
    "build", "check", "clippy", "fmt", "metadata", "test", "tree",
];
const ALLOWED_GIT_SUBCOMMANDS: &[&str] = &[
    "blame",
    "branch",
    "diff",
    "grep",
    "log",
    "ls-files",
    "rev-parse",
    "show",
    "status",
];

pub fn workspace_root() -> Result<PathBuf, String> {
    let cwd =
        std::env::current_dir().map_err(|e| format!("failed to get current directory: {e}"))?;
    cwd.canonicalize()
        .map_err(|e| format!("failed to resolve workspace root {}: {e}", cwd.display()))
}

pub fn resolve_existing_path(path: &Path) -> Result<PathBuf, String> {
    let root = workspace_root()?;
    let candidate = absolutize(path, &root);
    let canonical = candidate
        .canonicalize()
        .map_err(|e| format!("failed to resolve {}: {e}", candidate.display()))?;
    ensure_within_workspace(&canonical, &root)?;
    Ok(canonical)
}

pub fn resolve_existing_directory(path: &Path) -> Result<PathBuf, String> {
    let resolved = resolve_existing_path(path)?;
    if !resolved.is_dir() {
        return Err(format!("{} is not a directory", resolved.display()));
    }
    Ok(resolved)
}

pub fn resolve_path_for_write(path: &Path) -> Result<PathBuf, String> {
    let root = workspace_root()?;
    let candidate = absolutize(path, &root);
    if candidate.exists() {
        let canonical = candidate
            .canonicalize()
            .map_err(|e| format!("failed to resolve {}: {e}", candidate.display()))?;
        ensure_within_workspace(&canonical, &root)?;
        return Ok(canonical);
    }

    let (ancestor, remainder) = split_existing_ancestor(&candidate)?;
    let canonical_ancestor = ancestor
        .canonicalize()
        .map_err(|e| format!("failed to resolve {}: {e}", ancestor.display()))?;
    ensure_within_workspace(&canonical_ancestor, &root)?;

    let mut output = canonical_ancestor;
    for part in remainder {
        output.push(part);
    }
    Ok(output)
}

pub fn validate_shell_command(command: &str) -> Result<(), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("bash command cannot be empty".to_string());
    }
    if let Some(found) = FORBIDDEN_SHELL_FRAGMENTS
        .iter()
        .find(|f| trimmed.contains(*f))
    {
        return Err(format!(
            "bash command contains disallowed operator `{found}`. \
             Forbidden: ; ` $(). Allowed: && || | > <"
        ));
    }

    // Split on pipe/logical operators and validate each segment independently.
    let segments: Vec<&str> = trimmed
        .split(['|', '&', '>', '<'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    for segment in &segments {
        let mut parts = segment.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| "bash command cannot be empty".to_string())?;
        if !ALLOWED_SHELL_COMMANDS.contains(&program) {
            return Err(format!(
                "bash command `{program}` is not allowlisted. \
                 Allowed: cargo, git, python3, node, rg, grep, find, ls, cat, sed, awk, \
                 head, tail, make, mkdir, touch, cp, mv, rm, diff, and others"
            ));
        }

        match program {
            "cargo" => validate_cargo_command(&parts.collect::<Vec<_>>())?,
            "git" => validate_git_command(&parts.collect::<Vec<_>>())?,
            _ => {}
        }
    }

    Ok(())
}

fn validate_cargo_command(args: &[&str]) -> Result<(), String> {
    let Some(subcommand) = args.first().copied() else {
        return Err("cargo subcommand required".to_string());
    };
    if ALLOWED_CARGO_SUBCOMMANDS.contains(&subcommand) {
        Ok(())
    } else {
        Err(format!(
            "cargo subcommand `{subcommand}` is not allowlisted. \
             Allowed: build, check, clippy, fmt, metadata, test, tree"
        ))
    }
}

fn validate_git_command(args: &[&str]) -> Result<(), String> {
    let Some(subcommand) = args.first().copied() else {
        return Err("git subcommand required".to_string());
    };
    if ALLOWED_GIT_SUBCOMMANDS.contains(&subcommand) {
        Ok(())
    } else {
        Err(format!(
            "git subcommand `{subcommand}` is not allowlisted. \
             Allowed: blame, branch, diff, grep, log, ls-files, rev-parse, show, status"
        ))
    }
}

fn ensure_within_workspace(path: &Path, root: &Path) -> Result<(), String> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(format!(
            "path {} is outside the workspace root {}",
            path.display(),
            root.display()
        ))
    }
}

fn absolutize(path: &Path, root: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_path(path)
    } else {
        normalize_path(&root.join(path))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn split_existing_ancestor(path: &Path) -> Result<(PathBuf, Vec<OsString>), String> {
    let mut current = path;
    let mut remainder = Vec::new();
    while !current.exists() {
        let Some(name) = current.file_name() else {
            return Err(format!(
                "failed to resolve writable path {}",
                path.display()
            ));
        };
        remainder.push(name.to_os_string());
        current = current
            .parent()
            .ok_or_else(|| format!("failed to resolve writable path {}", path.display()))?;
    }
    remainder.reverse();
    Ok((current.to_path_buf(), remainder))
}
