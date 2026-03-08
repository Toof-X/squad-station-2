# Roadmap: Squad Station

## Milestones

- ✅ **v1.0 MVP** — Phases 1-3 (shipped 2026-03-06)
- ✅ **v1.1 Design Compliance** — Phases 4-6 (shipped 2026-03-08)
- 🚧 **v1.2 Distribution** — Phases 7-9 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-3) — SHIPPED 2026-03-06</summary>

- [x] Phase 1: Core Foundation (5/5 plans) — completed 2026-03-06
- [x] Phase 2: Lifecycle and Hooks (3/3 plans) — completed 2026-03-06
- [x] Phase 3: Views and TUI (2/2 plans) — completed 2026-03-06

</details>

<details>
<summary>✅ v1.1 Design Compliance (Phases 4-6) — SHIPPED 2026-03-08</summary>

- [x] Phase 4: Schema and Config Refactor (3/3 plans) — completed 2026-03-08
- [x] Phase 5: Feature Completion (2/2 plans) — completed 2026-03-08
- [x] Phase 6: Documentation (2/2 plans) — completed 2026-03-08

</details>

### 🚧 v1.2 Distribution (In Progress)

**Milestone Goal:** Make Squad Station installable by any developer in one command — `npm install -g squad-station` or `curl | sh`.

- [x] **Phase 7: CI/CD Pipeline** - GitHub Actions cross-compiles binaries for all 4 targets and publishes GitHub Releases (completed 2026-03-08)
- [ ] **Phase 8: npm Package** - npm package detects platform and installs the correct binary on `npm install -g squad-station`
- [ ] **Phase 9: Install Script and Docs** - curl | sh install alternative and README documenting all installation methods

## Phase Details

### Phase 7: CI/CD Pipeline
**Goal**: Automated releases deliver pre-built binaries for all platforms whenever a version tag is pushed
**Depends on**: Phase 6 (v1.1 complete baseline)
**Requirements**: CICD-01, CICD-02, CICD-03
**Success Criteria** (what must be TRUE):
  1. Pushing a `v*` tag triggers the GitHub Actions workflow without manual intervention
  2. The workflow produces four binaries: `darwin-arm64`, `darwin-x86_64`, `linux-arm64`, `linux-x86_64`
  3. A GitHub Release is created automatically with all four binaries attached as downloadable assets
  4. A developer can download a platform-specific binary directly from the GitHub Releases page
**Plans**: 1 plan
Plans:
- [ ] 07-01-PLAN.md — Cross-platform release workflow (GitHub Actions matrix build + GitHub Release creation)

### Phase 8: npm Package
**Goal**: Developers install Squad Station globally via npm and the correct binary lands in their PATH
**Depends on**: Phase 7
**Requirements**: NPM-01, NPM-02, NPM-03, NPM-04
**Success Criteria** (what must be TRUE):
  1. Running `npm install -g squad-station` completes without errors on macOS arm64, macOS x86_64, Linux arm64, and Linux x86_64
  2. After install, `squad-station --version` works in a new shell without any additional PATH configuration
  3. The postinstall script downloads the binary that matches the current OS and CPU architecture
  4. `package.json` correctly declares `bin`, `version`, `repository`, and `engines` fields
**Plans**: 2 plans
Plans:
- [ ] 08-01-PLAN.md — Create package.json, postinstall script, and JS bin wrapper
- [ ] 08-02-PLAN.md — Local smoke test: npm pack, install from tarball, human verify

### Phase 9: Install Script and Docs
**Goal**: A curl-based install alternative exists and README documents all ways to install and get started
**Depends on**: Phase 7
**Requirements**: INST-01, INST-02, INST-03, DOC-01, DOC-02, DOC-03
**Success Criteria** (what must be TRUE):
  1. Running `curl -fsSL <url> | sh` installs the binary to `/usr/local/bin` (or `~/.local/bin` fallback) on a machine without Node.js
  2. The install script detects platform and architecture, downloads the correct binary, and verifies it is executable before exiting
  3. README.md documents all three installation methods: npm, curl, and build from source
  4. README.md includes a quickstart showing `init`, `send`, and `signal` commands after installation
  5. README.md describes the project, links to PLAYBOOK.md, and provides an architecture overview
**Plans**: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Core Foundation | v1.0 | 5/5 | Complete | 2026-03-06 |
| 2. Lifecycle and Hooks | v1.0 | 3/3 | Complete | 2026-03-06 |
| 3. Views and TUI | v1.0 | 2/2 | Complete | 2026-03-06 |
| 4. Schema and Config Refactor | v1.1 | 3/3 | Complete | 2026-03-08 |
| 5. Feature Completion | v1.1 | 2/2 | Complete | 2026-03-08 |
| 6. Documentation | v1.1 | 2/2 | Complete | 2026-03-08 |
| 7. CI/CD Pipeline | v1.2 | 1/1 | Complete | 2026-03-08 |
| 8. npm Package | v1.2 | 0/2 | Not started | - |
| 9. Install Script and Docs | v1.2 | 0/? | Not started | - |
