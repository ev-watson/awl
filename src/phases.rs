use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Formulate,
    Plan,
    Execute,
    Verify,
    Complete,
}

impl Phase {
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Formulate => Some(Self::Plan),
            Self::Plan => Some(Self::Execute),
            Self::Execute => Some(Self::Verify),
            Self::Verify => Some(Self::Complete),
            Self::Complete => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Formulate => "Formulate",
            Self::Plan => "Plan",
            Self::Execute => "Execute",
            Self::Verify => "Verify",
            Self::Complete => "Complete",
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
}

impl PhaseState {
    pub fn new(task: &str) -> Self {
        Self {
            current: Phase::Formulate,
            task_description: task.to_string(),
            regression_count: 0,
            artifacts: Vec::new(),
            phase_notes: HashMap::new(),
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

    pub fn regress_to_execute(&mut self) -> Result<(), &'static str> {
        if self.regression_count >= 2 {
            return Err("max regressions reached (2); task cannot continue");
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
    }
}

pub enum GateSignal {
    Advance,
    Regress,
}

pub fn detect_gate(output: &str) -> Option<GateSignal> {
    let upper = output.to_uppercase();
    if upper.contains("FORMULATE_COMPLETE")
        || upper.contains("PLAN_COMPLETE")
        || upper.contains("EXECUTE_COMPLETE")
        || upper.contains("VERIFY_COMPLETE")
    {
        return Some(GateSignal::Advance);
    }
    if upper.contains("VERIFY_FAILED") {
        return Some(GateSignal::Regress);
    }
    None
}
