---
phase: 08-npm-package
verified: 2026-03-08T17:00:00Z
status: human_needed
score: 6/7 must-haves verified
re_verification: false
human_verification:
  - test: "Run npm install -g ./squad-station-0.1.0.tgz in a clean environment and confirm squad-station --version works in a new shell"
    expected: "Postinstall downloads the correct platform binary, sets it executable, and squad-station --version prints '0.1.0' without PATH modification"
    why_human: "Live network download from GitHub Releases cannot be verified statically. The 08-02 SUMMARY claims human approval occurred, but the claim cannot be re-confirmed programmatically."
---

# Phase 8: npm Package Verification Report

**Phase Goal:** Developers install Squad Station globally via npm and the correct binary lands in their PATH
**Verified:** 2026-03-08
**Status:** human_needed — all static/automated checks pass; one item requires live human confirmation
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | package.json exists with bin, version, repository, engines, files, and scripts fields | VERIFIED | File read confirms all 6 fields present; npm pack --dry-run lists it |
| 2 | scripts/postinstall.js detects OS+arch with correct maps (x64->x86_64), follows 301/302 redirects, and calls chmodSync 0755 | VERIFIED | Lines 12-13, 35, 62 confirmed by code read |
| 3 | bin/run.js has the node shebang on line 1, resolves binary via __dirname, and exec-spawns with stdio:inherit | VERIFIED | Lines 1, 7, 15 confirmed by code read |
| 4 | npm pack tarball contains exactly 3 files: package.json, bin/run.js, scripts/postinstall.js — no Rust source or planning files | VERIFIED | npm pack --dry-run output: 3 files confirmed |
| 5 | Both JS files are syntactically valid (no parse errors) | VERIFIED | node --check passes for both files |
| 6 | bin/run.js is executable (chmod +x applied) | VERIFIED | bin/run.js permissions: 0755 confirmed by ls -la |
| 7 | Local npm install completes successfully and squad-station --version works in a new shell | HUMAN NEEDED | 08-02 SUMMARY claims human approved; cannot re-verify live download statically |

**Score:** 6/7 truths verified (1 requires human)

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `package.json` | npm manifest with 6 required fields | VERIFIED | name: squad-station, version: 0.1.0, bin: ./bin/run.js, scripts.postinstall: node scripts/postinstall.js, files: [bin/, scripts/], engines: node>=14 |
| `scripts/postinstall.js` | Zero-dep binary downloader with platform detection, redirect following, chmodSync | VERIFIED | 71 lines, uses only https/fs/path built-ins. platformMap, archMap, 301/302 redirect loop, chmodSync 0o755 all present |
| `bin/run.js` | JS shim npm symlinks into PATH; delegates to native binary | VERIFIED | 22 lines, shebang line 1, spawnSync with stdio:inherit, __dirname binary resolution, exit code propagation |
| `bin/squad-station` | Native binary placed by postinstall at install time | HUMAN NEEDED | File not present in the source tree (expected — postinstall creates it at install time); human claimed confirmation per 08-02 SUMMARY |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| package.json | bin/run.js | `"bin": {"squad-station": "./bin/run.js"}` | VERIFIED | Exact pattern present in package.json line 10 |
| package.json | scripts/postinstall.js | `"scripts": {"postinstall": "node scripts/postinstall.js"}` | VERIFIED | Exact pattern present in package.json line 13 |
| scripts/postinstall.js | GitHub Releases download URL | `https.get` with `github.com/Toof-X/squad-station-2/releases/download/v{VERSION}/{assetName}` | VERIFIED | Line 53 constructs URL; downloadFile called with redirect following (lines 27-48) |
| bin/run.js | bin/squad-station | `spawnSync(binaryPath, ...)` where `binaryPath = path.join(__dirname, 'squad-station')` | VERIFIED | Lines 7, 15 confirmed |
| scripts/postinstall.js | bin/squad-station (chmod) | `fs.chmodSync(destPath, 0o755)` | VERIFIED | Line 62 confirmed |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| NPM-01 | 08-01, 08-02 | npm install -g squad-station installs the binary globally | SATISFIED (human gate) | package.json bin + postinstall wired correctly; 08-02 human approval claimed |
| NPM-02 | 08-01, 08-02 | Postinstall detects OS+arch, downloads correct binary from GitHub Releases | SATISFIED | platformMap/archMap confirmed; download URL construction verified; redirect following confirmed |
| NPM-03 | 08-01, 08-02 | Binary placed in npm bin dir and is executable without extra steps | SATISFIED (human gate) | bin/run.js is the npm bin shim (0755); chmodSync called on native binary at install |
| NPM-04 | 08-01 | package.json correctly configured (bin, version, repository, engines) | SATISFIED | All 4 named fields verified in package.json |

All 4 phase-8 requirement IDs from PLAN frontmatter are accounted for. No orphaned requirements — REQUIREMENTS.md traceability table maps NPM-01 through NPM-04 exclusively to Phase 8, and all are covered.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | No anti-patterns found |

Scanned `package.json`, `scripts/postinstall.js`, and `bin/run.js` for TODO, FIXME, XXX, HACK, PLACEHOLDER, empty returns, console-only implementations. None found. Both JS files contain substantive, complete implementations.

---

## Human Verification Required

### 1. End-to-End Install in a Clean Shell

**Test:** In a shell that has not run npm install during this session, run:
```
npm pack
npm install -g ./squad-station-0.1.0.tgz
```
Then open a NEW shell and run:
```
squad-station --version
squad-station --help
which squad-station
```

**Expected:**
- npm install exits 0 with "squad-station installed successfully" in output
- squad-station --version prints "0.1.0"
- squad-station --help lists all subcommands
- which squad-station returns a path (e.g., /opt/homebrew/bin/squad-station on macOS)
- On macOS arm64: `file $(which squad-station)` reports "Mach-O 64-bit executable arm64"

**Why human:** The postinstall script performs a live HTTPS download from GitHub Releases. This cannot be verified statically. The 08-02 SUMMARY documents human approval on 2026-03-08 for darwin-arm64, but a re-run of the live test is the only way to confirm the claim holds in a fresh environment.

---

## Gaps Summary

No blocking gaps. All static-verifiable must-haves pass:

- package.json: complete and correct
- scripts/postinstall.js: substantive implementation, all required behaviors present
- bin/run.js: correct shebang, wiring, exit code handling
- npm pack tarball: exactly 3 files, no extraneous content
- Key links: all 5 wiring points confirmed in source

The one unresolved item (live install + PATH verification) is inherently a runtime behavior. The 08-02 plan correctly classified it as a human-gated checkpoint, and the SUMMARY records human approval. The status is `human_needed` rather than `gaps_found` because no implementation defect was identified — the open item is confirmation of a runtime outcome.

---

_Verified: 2026-03-08T17:00:00Z_
_Verifier: Claude (gsd-verifier)_
