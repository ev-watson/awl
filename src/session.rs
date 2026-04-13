use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use rand::Rng;
use serde_json::Value;

use crate::phases::PhaseState;

/// Metadata header prefix — first line of the JSONL log.
const META_PREFIX: &str = "__claw_meta__";

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
        let suffix: u16 = rng.gen();
        let id = format!("{}-{suffix:04x}", chrono::Utc::now().timestamp());

        let mut dir = session_dir()?;
        fs::create_dir_all(&dir)?;
        dir.push(format!("{id}.jsonl"));

        Ok(Self { id, log_path: dir })
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
    let home = std::env::var("HOME")?;
    let mut dir = PathBuf::from(home);
    dir.push(".config/claw/sessions");
    Ok(dir)
}
