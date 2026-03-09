# Requirements: Squad Station

**Defined:** 2026-03-09
**Core Value:** Routing messages reliably between Orchestrator and agents — stateless CLI, no daemon.

## v1.3 Requirements

Requirements for v1.3: Antigravity & Hooks Optimization. Each maps to roadmap phases.

### Hooks

- [x] **HOOK-01**: `signal` command accepts `$TMUX_PANE` env var to auto-detect agent session name (hook requires no args beyond env var)
- [x] **HOOK-02**: `hooks/claude-code.sh` and `hooks/gemini-cli.sh` marked deprecated in file headers (kept as reference only)
- [ ] **HOOK-03**: `init` merges hook entries into existing `settings.json` with `.bak` backup before modification
- [ ] **HOOK-04**: `init` prints human-readable hook setup instructions when no `settings.json` exists

### Antigravity Provider

- [x] **AGNT-01**: `config.rs` supports `provider: antigravity` as valid orchestrator provider value with integration tests
- [x] **AGNT-02**: `signal.rs` skips `tmux send-keys` notification when orchestrator provider is `antigravity` (DB update only)
- [x] **AGNT-03**: `init.rs` skips tmux session creation for `antigravity` orchestrator — DB-only registration with clear log message

### IDE Context Generation

- [x] **AGNT-04**: `context.rs` generates `.agent/workflows/squad-delegate.md` for IDE orchestrators (delegation instructions + exact CLI commands)
- [x] **AGNT-05**: `context.rs` generates `.agent/workflows/squad-monitor.md` for IDE orchestrators (polling/monitoring with behavioral anti-context-decay rules)
- [x] **AGNT-06**: `context.rs` generates `.agent/workflows/squad-roster.md` for IDE orchestrators (agent list with names, models, descriptions)

### Safe Tmux Injection

- [ ] **TMUX-01**: `tmux.rs` implements `load-buffer`/`paste-buffer` pattern for safe multiline injection via temp file + cleanup
- [ ] **TMUX-02**: `send` command uses safe tmux adapter for all body content delivery (replaces direct `send-keys` for content)

### Documentation

- [ ] **DOCS-01**: `PLAYBOOK.md` rewritten with centralized hook setup documenting `squad-station signal $TMUX_PANE` inline command
- [ ] **DOCS-02**: `PLAYBOOK.md` documents Antigravity provider and IDE orchestrator mode
- [ ] **DOCS-03**: `PLAYBOOK.md` covers notification hook registration (deferred since GAP-04)

## v1.2 Requirements (Shipped)

### CI/CD

- [x] **CICD-01**: GitHub Actions workflow builds Rust binary for 4 targets: `darwin-arm64`, `darwin-x86_64`, `linux-arm64`, `linux-x86_64`
- [x] **CICD-02**: Workflow triggers on git tag push (`v*`) and creates a GitHub Release
- [x] **CICD-03**: GitHub Release has 4 pre-built binary assets attached (one per target)

### npm Package

- [x] **NPM-01**: `npm install -g squad-station` installs the binary globally
- [x] **NPM-02**: Postinstall script detects OS + CPU architecture and downloads correct binary from GitHub Releases
- [x] **NPM-03**: Binary placed in npm bin directory and is executable without extra steps
- [x] **NPM-04**: `package.json` correctly configured (`bin`, `version`, `repository`, `engines`)

### Install Script

- [x] **INST-01**: `curl -fsSL https://... | sh` installs the binary to `/usr/local/bin` (or `~/.local/bin` fallback)
- [x] **INST-02**: Install script detects platform + arch, downloads correct binary from GitHub Releases
- [x] **INST-03**: Install script verifies download succeeded and binary is executable

### Documentation (v1.2)

- [x] **DOC-01**: README.md documents all installation methods (npm, curl, build from source)
- [x] **DOC-02**: README.md includes a quickstart — first steps after install
- [x] **DOC-03**: README.md includes project description, architecture overview, link to PLAYBOOK.md

## Future Requirements

### Verification / Integrity

- **VER-01**: Install script verifies checksum of downloaded binary (SHA256)
- **VER-02**: GitHub Release includes `checksums.txt` with SHA256 for all assets

### Extended Distribution

- **DIST-01**: Homebrew formula for `brew install squad-station`
- **DIST-02**: AUR package for Arch Linux users

## Out of Scope

| Feature | Reason |
|---------|--------|
| Task management / workflow logic | Orchestrator AI responsibility |
| Web UI / browser dashboard | TUI sufficient, complexity not justified |
| Agent-to-agent direct messaging | All communication routes through orchestrator |
| Git conflict resolution | Orchestrator sequences work to avoid |
| Windows support | tmux not available on Windows; out of scope by architecture |
| Checksum verification | Deferred to v1.4 |
| Homebrew tap | Additional maintenance burden — defer to v1.4 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| HOOK-01 | Phase 10 | Complete |
| HOOK-02 | Phase 10 | Complete |
| HOOK-03 | Phase 12 | Pending |
| HOOK-04 | Phase 12 | Pending |
| AGNT-01 | Phase 11 | Complete |
| AGNT-02 | Phase 11 | Complete |
| AGNT-03 | Phase 11 | Complete |
| AGNT-04 | Phase 12 | Complete |
| AGNT-05 | Phase 12 | Complete |
| AGNT-06 | Phase 12 | Complete |
| TMUX-01 | Phase 13 | Pending |
| TMUX-02 | Phase 13 | Pending |
| DOCS-01 | Phase 13 | Pending |
| DOCS-02 | Phase 13 | Pending |
| DOCS-03 | Phase 13 | Pending |

**Coverage:**
- v1.3 requirements: 15 total
- Mapped to phases: 15
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-09*
*Last updated: 2026-03-09 after initial definition*
