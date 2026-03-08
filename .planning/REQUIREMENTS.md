# Requirements: Squad Station

**Defined:** 2026-03-08
**Core Value:** Routing messages reliably between Orchestrator and agents — all via stateless CLI commands, no daemon

## v1.2 Requirements

Requirements for v1.2 Distribution milestone. Each maps to roadmap phases.

### CI/CD

- [ ] **CICD-01**: GitHub Actions workflow builds Rust binary for 4 targets: `darwin-arm64`, `darwin-x86_64`, `linux-arm64`, `linux-x86_64`
- [ ] **CICD-02**: Workflow triggers on git tag push (`v*`) and creates a GitHub Release
- [ ] **CICD-03**: GitHub Release has 4 pre-built binary assets attached (one per target)

### npm Package

- [ ] **NPM-01**: `npm install -g squad-station` installs the binary globally
- [ ] **NPM-02**: Postinstall script detects OS + CPU architecture and downloads the correct binary from GitHub Releases
- [ ] **NPM-03**: Binary is placed in the npm bin directory and is executable without extra steps
- [ ] **NPM-04**: `package.json` is correctly configured (`bin`, `version`, `repository`, `engines`)

### Install Script

- [ ] **INST-01**: `curl -fsSL https://... | sh` installs the binary to `/usr/local/bin` (or `~/.local/bin` fallback)
- [ ] **INST-02**: Install script detects platform + arch, downloads correct binary from GitHub Releases
- [ ] **INST-03**: Install script verifies download succeeded and binary is executable

### Documentation

- [ ] **DOC-01**: README.md documents all installation methods (npm, curl, build from source)
- [ ] **DOC-02**: README.md includes a quickstart — first steps after install (init, send, signal)
- [ ] **DOC-03**: README.md includes project description, architecture overview, and link to PLAYBOOK.md

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
| Windows support | tmux not available on Windows; out of scope by architecture |
| Checksum verification | Good practice but adds complexity — defer to v1.3 |
| Homebrew tap | Additional maintenance burden — defer to v1.3 |
| Auto-update mechanism | Complexity not justified for v1.2 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CICD-01 | — | Pending |
| CICD-02 | — | Pending |
| CICD-03 | — | Pending |
| NPM-01 | — | Pending |
| NPM-02 | — | Pending |
| NPM-03 | — | Pending |
| NPM-04 | — | Pending |
| INST-01 | — | Pending |
| INST-02 | — | Pending |
| INST-03 | — | Pending |
| DOC-01 | — | Pending |
| DOC-02 | — | Pending |
| DOC-03 | — | Pending |

**Coverage:**
- v1.2 requirements: 13 total
- Mapped to phases: 0
- Unmapped: 13 ⚠️

---
*Requirements defined: 2026-03-08*
*Last updated: 2026-03-08 after initial definition*
