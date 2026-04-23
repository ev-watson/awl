use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Formulate,
    Plan,
    Execute,
    Verify,
    Complete,
    NeedsHuman,
}

impl Phase {
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Formulate => Some(Self::Plan),
            Self::Plan => Some(Self::Execute),
            Self::Execute => Some(Self::Verify),
            Self::Verify => Some(Self::Complete),
            Self::Complete | Self::NeedsHuman => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Formulate => "Formulate",
            Self::Plan => "Plan",
            Self::Execute => "Execute",
            Self::Verify => "Verify",
            Self::Complete => "Complete",
            Self::NeedsHuman => "NeedsHuman",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    pub current: Phase,
    pub task_description: String,
    pub regression_count: u8,
    pub artifacts: Vec<String>,
    pub phase_notes: HashMap<String, String>,
    #[serde(default)]
    pub persona: Option<String>,
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub ideas: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

impl PhaseState {
    pub fn new(task: &str) -> Self {
        Self {
            current: Phase::Formulate,
            task_description: task.to_string(),
            regression_count: 0,
            artifacts: Vec::new(),
            phase_notes: HashMap::new(),
            persona: None,
            goal: None,
            ideas: Vec::new(),
            evidence: Vec::new(),
        }
    }

    pub fn advance(&mut self) -> Option<Phase> {
        if let Some(next) = self.current.next() {
            self.current = next;
            Some(next)
        } else {
            None
        }
    }

    const MAX_REGRESSIONS: u8 = 2;

    pub fn regress_to_execute(&mut self) -> Result<(), &'static str> {
        if self.regression_count >= Self::MAX_REGRESSIONS {
            return Err("max regressions reached; task cannot continue");
        }
        self.current = Phase::Execute;
        self.regression_count += 1;
        Ok(())
    }
}

pub fn phase_system_prompt(phase: Phase) -> &'static str {
    match phase {
        Phase::Formulate => {
            "You are in the FORMULATE phase.
Define the problem, constraints, required files/context, and ambiguity blockers.
Do not write code yet. Signal completion with FORMULATE_COMPLETE."
        }
        Phase::Plan => {
            "You are in the PLAN phase.
Produce an ordered implementation plan with file-level changes and dependencies.
Do not write code yet. Signal completion with PLAN_COMPLETE."
        }
        Phase::Execute => {
            "You are in the EXECUTE phase.
Implement the plan using tools. Read before editing. Verify as you go.
Signal completion with EXECUTE_COMPLETE."
        }
        Phase::Verify => {
            "You are in the VERIFY phase.
Run tests/checks against acceptance criteria and edge cases.
If all good, output VERIFY_COMPLETE.
If issues remain, output VERIFY_FAILED with details."
        }
        Phase::Complete => "Task is complete.",
        Phase::NeedsHuman => "Task requires human review before continuing.",
    }
}

pub enum GateSignal {
    Advance,
    Regress,
}

pub fn detect_gate(phase: Phase, output: &str) -> Option<GateSignal> {
    let expected_advance = match phase {
        Phase::Formulate => Some("FORMULATE_COMPLETE"),
        Phase::Plan => Some("PLAN_COMPLETE"),
        Phase::Execute => Some("EXECUTE_COMPLETE"),
        Phase::Verify => Some("VERIFY_COMPLETE"),
        Phase::Complete | Phase::NeedsHuman => None,
    }?;
    let upper = output.to_ascii_uppercase();
    for line in upper.lines() {
        let trimmed = line.trim();
        if phase == Phase::Verify && trimmed.contains("VERIFY_FAILED") {
            return Some(GateSignal::Regress);
        }
        if trimmed.contains(expected_advance) {
            return Some(GateSignal::Advance);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_human_phase() {
        assert_eq!(Phase::NeedsHuman.name(), "NeedsHuman");
        assert_eq!(Phase::NeedsHuman.next(), None);
    }

    #[test]
    fn test_detect_gate_is_phase_aware() {
        assert!(matches!(
            detect_gate(Phase::Formulate, "notes\nFORMULATE_COMPLETE"),
            Some(GateSignal::Advance)
        ));
        assert!(detect_gate(Phase::Formulate, "VERIFY_COMPLETE").is_none());
        assert!(matches!(
            detect_gate(Phase::Verify, "result: VERIFY_FAILED due to clippy"),
            Some(GateSignal::Regress)
        ));
        assert!(detect_gate(Phase::Complete, "VERIFY_COMPLETE").is_none());
    }
}
