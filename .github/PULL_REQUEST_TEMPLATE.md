## Summary

- What changed
- Why it changed

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo package --no-verify --list`

## Checklist

- [ ] I updated docs for any user-facing behavior change.
- [ ] I kept the project local-first and did not add a cloud dependency.
- [ ] I did not widen workspace or shell access without explaining why.
