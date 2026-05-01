# Conventions and Notation Map

**Analysis Date:** 2026-05-01

## Reference Context

- Active Reference Registry: none confirmed in `GPD/state.json`; `GPD/state.json` is missing and only `GPD/state.json.lock` exists.
- Must-read references: none supplied.
- Prior outputs and known-good baselines: none supplied by the machine-readable intake.
- Stable knowledge documents: none found in the active runtime context.
- Scope note: this repository is a local-first Rust coding-agent CLI (`awl`), not a physics derivation project. Physics-specific convention slots are therefore marked absent rather than inferred.

## Artifact Inventory

- `README.md`: public product and developer documentation, including configuration precedence, environment variables, command examples, MCP guidance, and local quality gates.
- `Cargo.toml`: Rust package metadata, dependency declarations, release profile, and lint policy.
- `src/*.rs`: Rust implementation of CLI, dispatch, safety, MCP server, phase loop, repo map, session, and config behavior.
- `scripts/dispatch_eval.sh`, `scripts/dispatch_cost_report.py`: dispatch validation and cost-accounting scripts.
- `experiments/README.md`, `experiments/run_awl_arm.sh`, `experiments/tally.py`, `experiments/tasks/*`: A/B token-savings experiment harness.
- `.github/workflows/ci.yml`: CI quality gates on Ubuntu and macOS.
- No `.tex`, `.bib`, Jupyter, Mathematica, Maple, or physics manuscript files were found by repository scan.

## Physics Convention Status

| Convention | Status | Evidence / Notes |
|---|---|---|
| Unit system | Not applicable to physics units | No natural, SI, CGS, lattice, or dimensional convention appears in inspected source or docs. Operational quantities are tokens, milliseconds, chars, paths, and model levels. |
| Metric signature | Missing / not applicable | No Lorentzian or Euclidean metric conventions found. |
| Fourier transform convention | Missing / not applicable | No Fourier transforms or transform macros found. |
| Index convention | Not applicable to tensor indices | No Einstein summation, tensor index placement, covariant/contravariant notation, or tensor macros found. Code uses ordinary zero-based collection indexing and one-based display line tags where documented by tests. |
| Coordinate labeling | Not applicable to spacetime coordinates | The meaningful "coordinates" are workspace paths, config paths, line numbers, model levels, and CLI/MCP field names. |
| Coupling constants | Missing / not applicable | No physics coupling definitions such as `g`, `alpha`, or `g^2/(4pi)` found. |
| Wick rotation | Missing / not applicable | No Euclidean continuation or path-integral convention found. |
| Gauge, spinor, state normalization, renormalization scheme | Missing / not applicable | No gauge theory, spinor basis, quantum state normalization, or renormalization artifacts found. |

## Software Unit and Quantity Conventions

- Time is represented in milliseconds in dispatch telemetry and scripts:
  - `scripts/dispatch_eval.sh` records `elapsed_ms` and `model_elapsed_ms`.
  - `experiments/run_awl_arm.sh` records `wall_ms` and model elapsed milliseconds.
  - `src/dispatch.rs` uses a verify timeout constant in milliseconds and reports timeout text in `ms`.
- Token accounting distinguishes prompt/input, completion/output, and total tokens:
  - `scripts/dispatch_cost_report.py` reads `prompt_tokens`, `input_tokens`, `completion_tokens`, `output_tokens`, and `total_tokens`.
  - `experiments/tally.py` currently compares aggregate frontier tokens against Awl tokens.
- Model levels are numeric operational tiers, not physical quantum numbers:
  - level 2 is implementation (`DEFAULT_IMPLEMENTATION_MODEL` in `src/defaults.rs`).
  - level 3 is verification (`DEFAULT_VERIFICATION_MODEL` in `src/defaults.rs`).
- Size limits are character counts:
  - dispatch context and return compaction use `max_return_chars`, per-file context limits, and total context character limits in `src/dispatch.rs`.
- Paths are workspace-scoped:
  - `src/safety.rs` canonicalizes read/write paths and rejects paths outside the workspace root.
  - config/session paths are resolved separately in `src/config.rs` and `src/doctor.rs`.

## Naming Conventions

### Rust Code

- Functions, variables, module filenames, and JSON field struct members use `snake_case`, e.g. `configured_ollama_base_url`, `verify_command`, `target_path`, `context_paths`.
- Types and enum variants use Rust `PascalCase`, e.g. `UserConfig`, `PhaseState`, `NeedsHuman`, `GateSignal`.
- Constants and environment variable names use uppercase identifiers:
  - Rust constants: `DEFAULT_AGENT_MODEL`, `DEFAULT_OLLAMA_BASE_URL`, `ENABLE_MCP_AGENT_ENV`.
  - Environment variables documented in `README.md`: `AWL_AGENT_MODEL`, `AWL_IMPLEMENTATION_MODEL`, `AWL_VERIFICATION_MODEL`, `AWL_SESSIONS_DIR`, `AWL_MCP_CONFIG`, `AWL_CONFIG_PATH`, `AWL_CONFIG_DIR`.
- Public command surface uses kebab-case CLI flags:
  - examples include `--target-path`, `--verify`, `--max-attempts`, `--auto-repomap`, `--max-return-chars`.

### JSON and MCP Payloads

- JSON fields use `snake_case`, matching Rust serde structs and script parsers:
  - `target_path`, `target_files`, `context_paths`, `verify_command`, `max_attempts`, `max_return_chars`, `auto_repomap`, `files_changed`, `files_intended`, `checks_passed`.
- Dispatch result semantics distinguish trusted side effects from model intent:
  - `files_changed` / compatibility `files_modified`: actual writes in apply mode.
  - `files_intended`: model-claimed files in non-apply mode.
- Telemetry and scripts consistently use `dispatch_id`, `log_path`, `attempts`, `status`, and token fields.

### Documents and Experiments

- Experiment task directories use numeric prefixes and short snake-case descriptors: `experiments/tasks/01_string_helper`, `02_validate_input`, `03_fix_off_by_one`.
- Python sandbox functions in experiment tasks use normal Python `snake_case`: `capitalize_each_line`, `validate_email`, `moving_average`.
- Documentation names are uppercase for policy/report files (`README.md`, `SECURITY.md`, `CHANGELOG.md`) and descriptive report titles (`UPDATED_PROGRESS_REPORT.md`, `REPORT_DISPATCH_RELIABILITY.md`).

## Ordering and Sign Conventions

- There are no physics sign conventions.
- Agent execution order is explicitly named in `README.md` and `src/phases.rs`:
  - `Formulate -> Plan -> Execute -> Verify -> Complete`.
  - A verify failure regresses to `Execute` until `MAX_REGRESSIONS` is reached.
- Gate signals are uppercase strings:
  - `FORMULATE_COMPLETE`, `PLAN_COMPLETE`, `EXECUTE_COMPLETE`, `VERIFY_COMPLETE`, `VERIFY_FAILED`, `TASK_COMPLETE`.
- Shell safety parsing in `src/safety.rs` treats shell segments around pipes, logical operators, and redirects as separately validated command segments.

## Coordinate and Index-Like Conventions

- Workspace coordinates are filesystem paths rooted at the current workspace:
  - `src/safety.rs` resolves existing paths and write targets against the canonical workspace root.
  - writes outside the root are rejected.
- Hashline editing uses line-number-plus-hash anchors:
  - `src/hashline.rs` builds lookup keys like `LINE:HASH`.
  - tests show line display numbers start at 1.
- Slice/window logic appears in experiment tasks, not core mathematical formalism:
  - `experiments/tasks/03_fix_off_by_one/setup.sh` intentionally seeds a Python range off-by-one bug for validation.

## Dependency and Configuration Conventions

- Configuration precedence is documented in `README.md`:
  - CLI flags, then environment variables, then user config, then built-in defaults.
- Default local model identities are declared in `src/defaults.rs`:
  - agent: `qwen2.5-coder:14b`.
  - implementation: `qwen2.5-coder:7b-instruct-q4_K_M`.
  - verification: `qwen2.5-coder:3b-instruct-q4_K_M`.
- The default Ollama OpenAI-compatible endpoint is `http://127.0.0.1:11434/v1`.
- `Cargo.toml` pins the Rust edition to 2021 and forbids unsafe code via `[lints.rust] unsafe_code = "forbid"`.

## Notation Risks and Missing Convention Controls

- No project `convention_lock` is available because `GPD/state.json` is missing. There is no authoritative physics convention baseline to validate against.
- No LaTeX macros or notation preambles exist to inspect; therefore no equation-level notation map can be produced.
- `experiments/tally.py` uses a blended `--cost-per-mtok` convention, while `REPORT_DISPATCH_RELIABILITY.md` recommends separate input/output pricing. This is a documented methodology mismatch to resolve before using cost estimates as final evidence.
- `src/safety.rs` error text allows `&&`, `||`, `|`, `>`, `<` while forbidding semicolon, backtick, command substitution, and newline. This is a software safety convention, not a shell-language proof; edge-case parsing should remain covered by tests if the allowlist changes.
- The project uses local model names as operational identifiers. Any report comparing models should record exact model strings and not collapse `7b`, `7b-instruct-q4_K_M`, and `14b` into informal labels.

---

_Methodology convention map generated from inspected repository artifacts on 2026-05-01._
