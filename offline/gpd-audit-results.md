# GPD Server Audit (2026-04-13)

| Server | Tools | Verdict | Reason |
|---|---|---|---|
| gpd-conventions | 6 | **Keep** | Domain logic: convention locking, validation, drift detection, subfield defaults for 14 physics domains. Useful for physics projects with convention enforcement. |
| gpd-patterns | 5 | **Keep** | Domain logic: physics error pattern library with search, seeding, and confidence promotion. Contains bootstrap patterns for sign errors, factor errors, convention pitfalls. |
| gpd-protocols | 4 | **Keep** | Domain logic: physics computation protocols (perturbation theory, renormalization group, etc.) with checkpoints and auto-routing. Structured procedural knowledge. |
| gpd-verification | 9 | **Keep** | Domain logic: dimensional analysis, limiting case checks, symmetry checks, verification coverage gap analysis. Heaviest domain value — 9 tools covering structured physics verification. |
| gpd-errors | 5 | **Keep** | Domain logic: 104 physics error classes with detection strategies and traceability matrix mapping errors to verification checks. |
| gpd-state | 7 | **Drop** | State management: get/set project state, phase tracking, progress, health checks. Already reimplemented in `phases.rs` (PhaseState + Session). Adds no domain knowledge. |
| gpd-skills | 4 | **Drop** | Skill registry: list/get/route canonical skills. Already handled by claw's tool registry. No physics domain content — just routing metadata. |

## Summary

Keep 5 servers (29 tools), drop 2 (11 tools). The retained servers provide physics domain knowledge that cannot be derived from the claw codebase. The dropped servers duplicate infrastructure already in Rust.
