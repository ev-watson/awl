# Contributing

## Setup

1. Install Rust stable and [Ollama](https://ollama.com).
2. Clone the repository and build locally:

```bash
git clone https://github.com/ev-watson/awl.git
cd awl
cargo build
```

3. Configure a local runtime if you want to exercise the full CLI:

```bash
awl init --profile lite --no-check
ollama pull qwen2.5-coder:7b-instruct-q4_K_M
ollama pull qwen2.5-coder:3b-instruct-q4_K_M
awl doctor
```

## Quality Gates

Run these before opening a pull request:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo package --no-verify --list
```

## Pull Requests

- Open a pull request against `main`.
- Keep changes scoped and explain user-facing effects clearly.
- Include reproduction steps for bug fixes.
- Update `README.md` when install, config, or command behavior changes.
- Expect CI to pass before merge.

If you are changing command-line behavior, configuration resolution, or safety boundaries, explain the tradeoff in the pull request description.

## Scope

- Keep changes local-first. Awl should remain usable without cloud dependencies.
- Preserve workspace containment and avoid widening the shell/file surface casually.
- Prefer clear, testable code over additional abstraction.
- Document user-facing behavior changes in `README.md`.

## Releases

- Update `CHANGELOG.md` and package metadata for the release version.
- Tag releases as `vX.Y.Z`.
- Pushing the tag triggers the GitHub Actions release workflow and uploads prebuilt archives.
