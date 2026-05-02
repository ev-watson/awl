# Conventions: Awl Frontier-Token Savings Study

<!-- ASSERT_CONVENTION: custom:project_type=software_systems_empirical_measurement_nonphysical, custom:token_units=provider_reported_tokens_split_input_output, custom:cost_units=usd_per_million_frontier_tokens_split_c_in_c_out, custom:time_units=wall_time_seconds_duration_ms_only_for_ms_fields, custom:model_naming=exact_ollama_model_tag_plus_fixed_arm_label, custom:result_schema_naming=snake_case_json_csv_fields, custom:failure_category_enum=format,schema,preflight,verify,timeout,network,model,unknown, custom:experiment_comparability=same_frozen_task_ids_and_verifier_semantics_across_arms, custom:git_ci_evidence=command_exit_status_date_branch_commit_and_ci_check_name -->

**Status:** initial approved custom conventions locked on 2026-05-01.

This project is an empirical software-systems measurement study of Awl's
bounded local coding delegation workflow. It is not a physics derivation,
field-theory calculation, or physical measurement campaign. The conventions
below define how experiment data, model identities, token accounting, cost
accounting, failure categories, and validation evidence must be represented.

## Canonical Physics Conventions

The canonical physics convention fields are intentionally unset. Metric
signature, Fourier convention, natural units, gauge choice, regularization
scheme, renormalization scheme, coordinate system, spin basis, state
normalization, coupling convention, index positioning, time ordering,
commutation convention, Levi-Civita sign, generator normalization, covariant
derivative sign, gamma matrix convention, and creation/annihilation ordering
are not applicable to this nonphysical software-systems project.

Do not lock placeholder values such as "not applicable" into canonical physics
fields, and do not introduce fallback physics defaults. If a later phase
introduces an actual physics derivation or physical unit model, it must request
a scoped convention update instead of inferring one from this file.

## Approved Custom Convention Lock

| Key | Locked value | Meaning |
| --- | --- | --- |
| `custom:project_type` | `software_systems_empirical_measurement_nonphysical` | The study measures a software-agent workflow, not a physical system. |
| `custom:token_units` | `provider_reported_tokens_split_input_output` | Token counts are accepted only when split into provider-reported input and output fields or traceably mapped equivalents. |
| `custom:cost_units` | `usd_per_million_frontier_tokens_split_c_in_c_out` | Frontier costs are reported in USD per million tokens with separate input and output rates, conventionally `c_in` and `c_out`. |
| `custom:time_units` | `wall_time_seconds_duration_ms_only_for_ms_fields` | Human-facing durations use wall-time seconds unless a field name explicitly ends in `_ms` or otherwise declares milliseconds. |
| `custom:model_naming` | `exact_ollama_model_tag_plus_fixed_arm_label` | Model evidence must preserve the exact Ollama tag and the fixed experiment arm label, such as `7B-only` or `14B-only`. |
| `custom:result_schema_naming` | `snake_case_json_csv_fields` | JSON and CSV result fields use `snake_case`. |
| `custom:failure_category_enum` | `format,schema,preflight,verify,timeout,network,model,unknown` | Terminal dispatch failures must map to one of these categories. |
| `custom:experiment_comparability` | `same_frozen_task_ids_and_verifier_semantics_across_arms` | Baseline, 7B-only, and 14B-only comparisons require identical frozen task IDs and verifier semantics. |
| `custom:git_ci_evidence` | `command_exit_status_date_branch_commit_and_ci_check_name` | Verification evidence must record command, exit status, date, branch, commit, and CI check name when applicable. |

## Measurement Quantities

Use these symbols only as shorthand in reports; machine-readable data should use
the field names below.

| Quantity | Preferred field or symbol | Unit / type | Example test value |
| --- | --- | --- | --- |
| Frontier input tokens | `frontier_input_tokens`, `input_tokens`, or `prompt_tokens` | provider-reported tokens | `12000` |
| Frontier output tokens | `frontier_output_tokens`, `output_tokens`, or `completion_tokens` | provider-reported tokens | `1800` |
| Frontier token reduction | `S` or `frontier_token_reduction_pct` | percent | `27.5` means 27.5 percent reduction. |
| Awl pass rate | `p` or `pass_rate_pct` | percent | `70.0` means 7 of 10 tasks passed. |
| Input cost rate | `c_in` or `cost_per_mtok_input_usd` | USD per million input tokens | `5.0` |
| Output cost rate | `c_out` or `cost_per_mtok_output_usd` | USD per million output tokens | `25.0` |
| Wall time | `wall_time_s` | seconds | `42.317` |
| Millisecond duration | fields ending in `_ms`, such as `wall_ms` | milliseconds | `42317` |
| Task count | `task_count` or `N` | count | `10` |

Cost calculation convention:

```text
frontier_cost_usd =
  (input_tokens / 1_000_000) * c_in
  + (output_tokens / 1_000_000) * c_out
```

Example:

```json
{
  "input_tokens": 12000,
  "output_tokens": 1800,
  "cost_per_mtok_input_usd": 5.0,
  "cost_per_mtok_output_usd": 25.0,
  "frontier_cost_usd": 0.105
}
```

Do not use a blended single token rate for final Step 1 claims after split
input/output accounting is available. Legacy total-token fields may be kept for
readability, but they are not sufficient evidence for the savings claim.

## Model Naming

Every model-comparison artifact must include both:

1. The fixed experiment arm label: `direct-frontier`, `7B-only`, or `14B-only`.
2. The exact local model tag when Awl is used, for example
   `qwen2.5-coder:7b-instruct-q4_K_M`.

Valid example:

```json
{
  "arm": "7B-only",
  "local_model": "qwen2.5-coder:7b-instruct-q4_K_M",
  "task_id": "01_string_helper"
}
```

Invalid example:

```json
{
  "arm": "local",
  "local_model": "qwen"
}
```

Rationale: informal model names make 7B, quantized 7B instruct, 14B, and future
tags ambiguous, which can invalidate pass-rate and cost comparisons.

## Result Schema Naming

JSON and CSV fields use `snake_case`. Preferred field examples:

- `task_id`
- `arm`
- `local_model`
- `status`
- `failure_category`
- `input_tokens`
- `output_tokens`
- `wall_time_s`
- `wall_ms`
- `attempts`
- `verify_command`
- `ci_check_name`

Avoid camelCase, PascalCase, and mixed spellings in new result artifacts.
Compatibility readers may accept older names, but reports should normalize to
the locked `snake_case` convention.

## Failure Categories

Use exactly one of the locked enum values for each terminal dispatch failure:

| Category | Use when |
| --- | --- |
| `format` | Model output cannot be parsed as the expected transport format. |
| `schema` | Model output parses but violates the expected JSON schema or required fields. |
| `preflight` | A pre-apply safety, path, command, or workspace check fails. |
| `verify` | The task verifier, test command, or post-apply validation fails. |
| `timeout` | The worker, apply step, verifier, or harness exceeds the configured time limit. |
| `network` | Ollama or another required local endpoint is unreachable or returns a transport error. |
| `model` | The selected local model refuses, produces unusable content after schema handling, or reports a model/runtime error not better classified above. |
| `unknown` | The failure is terminal but evidence is insufficient for a more specific category. |

Example test records:

```json
{"task_id": "01_string_helper", "status": "fail", "failure_category": "verify"}
{"task_id": "mcp_schema_case", "status": "fail", "failure_category": "schema"}
{"task_id": "ollama_down_case", "status": "fail", "failure_category": "network"}
```

Do not silently omit `failure_category` on failures. If classification evidence
is weak, use `unknown` and preserve the log path or error text needed to improve
the taxonomy later.

## Experiment Comparability

The direct frontier baseline, 7B-only Awl arm, and 14B-only Awl arm are
comparable only when all of the following hold:

- The task ID set is identical across arms.
- Task definitions are frozen before running the comparison.
- Verifier commands and pass/fail semantics are identical across arms.
- Post hoc task exclusions are recorded as exclusions and do not disappear from
  the denominator.
- Input/output token accounting is available for paid frontier usage.

Minimum comparability test:

```text
sorted(task_id from baseline) == sorted(task_id from awl_7b) == sorted(task_id from awl_14b)
```

If a verifier changes after baseline collection, earlier rows are not directly
comparable unless rerun or explicitly marked as incompatible.

## Git, Local Verification, and CI Evidence

Evidence for reliability patches and reports must preserve:

- command run, for example `cargo test`;
- exit status, for example `0`;
- date, preferably ISO date such as `2026-05-01`;
- branch name;
- commit hash when available;
- CI check name when applicable, such as `checks (ubuntu-latest)`.

Example:

```json
{
  "command": "cargo test",
  "exit_status": 0,
  "date": "2026-05-01",
  "branch": "feature/reliability-patches",
  "commit": "abc1234",
  "ci_check_name": null
}
```

Rationale: a pass/fail claim without date, branch, commit, and check identity is
too weak to support the reliability-prep deliverable or later Step 1 report.

## Review Checklist

Before accepting any phase artifact as convention-compliant, check:

- Canonical physics convention fields remain unset unless a later scoped update
  explicitly introduces physical content.
- New JSON and CSV fields use `snake_case`.
- Token and cost evidence includes input/output splits, not only total tokens.
- Model comparisons include fixed arm labels and exact Ollama tags.
- Every terminal failure has one locked `failure_category`.
- Cross-arm comparisons use the same frozen task IDs and verifier semantics.
- Verification evidence includes command, exit status, date, branch, commit, and
  CI check name when applicable.
