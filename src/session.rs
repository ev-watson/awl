use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use rand::Rng;
use serde_json::Value;

use crate::config;
use crate::phases::PhaseState;

/// Metadata header prefix — first line of the JSONL log.
const META_PREFIX: &str = "__awl_meta__";

pub struct Session {
    pub id: String,
    log_path: PathBuf,
}

/// Parsed contents of a resumed session.
pub struct ResumedSession {
    pub session: Session,
    pub phase_state: Option<PhaseState>,
    pub messages: Vec<Value>,
}

impl Session {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut rng = rand::thread_rng();
        let dir = session_dir()?;
        fs::create_dir_all(&dir)?;

        for _ in 0..16 {
            let suffix: u32 = rng.gen();
            let id = format!(
                "{}-{suffix:08x}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            let log_path = dir.join(format!("{id}.jsonl"));
            match OpenOptions::new()
                .create_new(true)
                .append(true)
                .open(&log_path)
            {
                Ok(_) => return Ok(Self { id, log_path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
        }

        Err("failed to allocate a unique session id".into())
    }

    /// Write the phase state as a metadata header line at session start.
    pub fn write_metadata(&self, state: &PhaseState) -> Result<(), Box<dyn std::error::Error>> {
        let meta = serde_json::json!({
            META_PREFIX: true,
            "phase_state": state,
        });
        let encoded = serde_json::to_string(&meta)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        writeln!(file, "{encoded}")?;
        Ok(())
    }

    /// Resume from an existing session log.
    /// Extracts metadata (`PhaseState`) from the header line and conversation
    /// messages from all subsequent lines.
    pub fn resume(id: &str) -> Result<ResumedSession, Box<dyn std::error::Error>> {
        let mut path = session_dir()?;
        path.push(format!("{id}.jsonl"));
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);

        let mut phase_state: Option<PhaseState> = None;
        let mut messages = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line)?;

            // Check if this is a metadata line.
            if value.get(META_PREFIX).is_some() {
                if let Some(ps) = value.get("phase_state") {
                    phase_state = serde_json::from_value(ps.clone()).ok();
                }
                continue;
            }

            messages.push(value);
        }

        Ok(ResumedSession {
            session: Self {
                id: id.to_string(),
                log_path: path,
            },
            phase_state,
            messages,
        })
    }

    pub fn append(&self, message: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let encoded = serde_json::to_string(message)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        writeln!(file, "{encoded}")?;
        Ok(())
    }

    /// Update the phase state in the log. Appends a new metadata line —
    /// on resume, the last metadata line wins.
    pub fn update_metadata(&self, state: &PhaseState) -> Result<(), Box<dyn std::error::Error>> {
        self.write_metadata(state)
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

fn session_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    config::configured_sessions_dir()
}

/// Information about a stored session file.
pub struct SessionInfo {
    pub id: String,
    pub size_bytes: u64,
    pub modified: std::time::SystemTime,
}

/// List all session files, sorted by modification time (newest first).
pub fn list_sessions() -> Result<Vec<SessionInfo>, Box<dyn std::error::Error>> {
    let dir = session_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut sessions = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let meta = entry.metadata()?;
        let id = path
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().to_string());
        sessions.push(SessionInfo {
            id,
            size_bytes: meta.len(),
            modified: meta.modified()?,
        });
    }
    sessions.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(sessions)
}

/// Delete session files older than `max_age_days` days. Returns count of deleted files.
pub fn prune_sessions(max_age_days: u64) -> Result<usize, Box<dyn std::error::Error>> {
    let dir = session_dir()?;
    if !dir.exists() {
        return Ok(0);
    }
    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_days * 24 * 60 * 60);
    let mut deleted = 0;
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let meta = entry.metadata()?;
        let modified = meta.modified()?;
        if modified < cutoff {
            fs::remove_file(&path)?;
            deleted += 1;
        }
    }
    Ok(deleted)
}
