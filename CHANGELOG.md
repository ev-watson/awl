# Changelog

## 0.3.0 - 2026-04-23

### Added

- User configuration file support for endpoint, model, session, and MCP settings
- `awl init` for first-run setup and profile-based configuration
- Public installer script at `scripts/install.sh`
- GitHub Actions workflows for CI, tagged releases, and dependency updates
- Public repo hygiene docs: contributing guide and security policy

### Changed

- Runtime defaults now honor config and environment overrides consistently
- `awl doctor` validates the configured endpoint, configured models, and session directory
- Documentation now targets public installation and release usage instead of a local-only workflow

### Fixed

- Ollama endpoint handling now tolerates both OpenAI-style base URLs and `OLLAMA_HOST` style host values
- Session storage no longer assumes a single hardcoded directory
