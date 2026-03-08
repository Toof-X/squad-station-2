---
phase: 8
slug: npm-package
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-08
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Manual smoke testing + static JSON checks (no test framework install needed) |
| **Config file** | none — npm built-in tools only |
| **Quick run command** | `node -e "const p=require('./package.json'); ['bin','version','repository','engines'].forEach(f => { if(!p[f]) throw new Error('missing: '+f); }); console.log('package.json OK')"` |
| **Full suite command** | `npm pack --dry-run && node -e "const p=require('./package.json'); ['bin','version','repository','engines'].forEach(f => { if(!p[f]) throw new Error('missing: '+f); }); console.log('package.json OK')"` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run quick run command (package.json field check)
- **After every plan wave:** Run `npm pack --dry-run` to verify tarball contents
- **Before `/gsd:verify-work`:** Local `npm install -g .` smoke test must pass
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 8-01-01 | 01 | 1 | NPM-04 | static | `node -e "const p=require('./package.json'); ['bin','version','repository','engines'].forEach(f => { if(!p[f]) throw new Error('missing: '+f); }); console.log('OK')"` | ❌ W0 | ⬜ pending |
| 8-01-02 | 01 | 1 | NPM-02 | smoke | `node scripts/postinstall.js` (with real release) | ❌ W0 | ⬜ pending |
| 8-01-03 | 01 | 1 | NPM-03 | smoke | `squad-station --version` after local install | ❌ W0 | ⬜ pending |
| 8-01-04 | 01 | 1 | NPM-01 | smoke | `npm pack && npm install -g ./squad-station-*.tgz` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `package.json` — must be created at project root (Wave 1 deliverable)
- [ ] `scripts/postinstall.js` — must be created (Wave 1 deliverable)
- [ ] `bin/run.js` — must be created (Wave 1 deliverable)

*No test framework install needed — validation is smoke testing only; `npm pack` is built into npm.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `npm install -g squad-station` succeeds on macOS arm64 | NPM-01 | Requires real npm registry publish and real hardware | `npm install -g squad-station` in a clean shell; verify exit 0 |
| `npm install -g squad-station` succeeds on macOS x86_64 | NPM-01 | Requires real npm registry publish and real hardware | Same as above on x86_64 machine |
| `npm install -g squad-station` succeeds on Linux arm64 | NPM-01 | Requires real npm registry publish and real hardware | Same as above on Linux arm64 |
| `npm install -g squad-station` succeeds on Linux x86_64 | NPM-01 | Requires real npm registry publish and real hardware | Same as above on Linux x86_64 |
| `squad-station --version` works without extra PATH setup | NPM-03 | Requires post-install shell context | Open new shell, run `squad-station --version`, verify output without modifying PATH |
| Postinstall downloads correct platform binary | NPM-02 | Platform-specific; needs real GitHub Release assets | Install on each target platform, verify binary matches OS/arch |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
