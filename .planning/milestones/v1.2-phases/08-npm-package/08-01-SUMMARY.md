---
phase: 08-npm-package
plan: 01
subsystem: infra
tags: [npm, nodejs, postinstall, binary-distribution, github-releases]

# Dependency graph
requires:
  - phase: 07-ci-cd-pipeline
    provides: "GitHub Releases with squad-station-{os}-{arch} binaries at v{version} tags"
provides:
  - "package.json npm manifest with bin, postinstall, files whitelist"
  - "scripts/postinstall.js: cross-platform binary downloader with redirect following"
  - "bin/run.js: JS shim that npm symlinks into PATH as squad-station"
affects: [09-install-script-and-docs]

# Tech tracking
tech-stack:
  added: [npm-package, nodejs-https-module, nodejs-child_process]
  patterns: [postinstall-binary-download, zero-external-deps-installer, js-shim-for-native-binary]

key-files:
  created:
    - package.json
    - scripts/postinstall.js
    - bin/run.js
  modified: []

key-decisions:
  - "Version in package.json (0.1.0) must match Cargo.toml — postinstall reads it at runtime to construct download URL"
  - "archMap x64->x86_64 mapping is critical: Node.js uses x64, Phase 7 release binaries use x86_64"
  - "files whitelist [bin/, scripts/] excludes all Rust source, .planning/, and CI config from npm tarball"
  - "Zero external dependencies in postinstall.js: uses only built-in Node.js https, fs, path modules"
  - "Redirect following in downloadFile: GitHub Releases always returns 302 to S3 — without it download fails silently"

patterns-established:
  - "Binary naming: squad-station-{os}-{arch} — Phase 9 install script must use same convention"
  - "Version single-source-of-truth: package.json version field; postinstall reads it at runtime"
  - "JS shim pattern: bin/run.js resolves binary via __dirname, forwards all args and exit codes"

requirements-completed: [NPM-01, NPM-02, NPM-03, NPM-04]

# Metrics
duration: 1min
completed: 2026-03-08
---

# Phase 8 Plan 01: npm Package Manifest and Install Scripts Summary

**npm package scaffold with zero-dependency postinstall downloader: platform/arch detection, 301/302 redirect following, chmodSync, and JS shim wrapper for PATH integration**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-08T16:13:40Z
- **Completed:** 2026-03-08T16:14:48Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Created package.json with all required fields (bin, version, repository, engines, files, scripts) pointing to JS shim and postinstall script
- Created scripts/postinstall.js with OS/arch detection (darwin/linux, x64->x86_64/arm64), 301/302 redirect following for GitHub's S3 redirects, chmodSync 0755, and zero external dependencies
- Created bin/run.js as executable JS shim that npm symlinks into PATH, forwarding all args and exit codes to the native binary via spawnSync with stdio:inherit
- Verified npm pack --dry-run lists exactly 3 files (no Rust source, planning, or CI config)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create package.json** - `b141525` (feat)
2. **Task 2: Create postinstall script and bin wrapper** - `28c27fd` (feat)

## Files Created/Modified
- `package.json` - npm manifest: name, version, bin, postinstall, files whitelist, engines, license
- `scripts/postinstall.js` - downloads correct platform binary from GitHub Releases at install time
- `bin/run.js` - thin JS shim; npm symlinks this as `squad-station` in PATH; delegates to native binary

## Decisions Made
- Version in package.json set to 0.1.0 matching Cargo.toml; postinstall reads it at runtime via `require('../package.json').version` so version is always in sync
- archMap uses `x64 -> x86_64` mapping because Node.js reports `process.arch` as `x64` but Phase 7 release binaries are named with `x86_64`
- Zero external dependencies: built-in https, fs, path modules only — no npm install step needed before postinstall runs
- Redirect following up to 5 hops: GitHub Releases 302 to S3 is required for download to succeed

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- npm package scaffold complete; ready for Phase 9 (install script and documentation)
- Phase 9 curl|sh install script must use same binary naming convention: squad-station-{os}-{arch}
- No blockers. Phase 7 (GitHub Releases CI workflow) must be verified working before `npm install -g squad-station` can be tested end-to-end.

## Self-Check: PASSED

- package.json: FOUND
- scripts/postinstall.js: FOUND
- bin/run.js: FOUND
- commit b141525 (feat: package.json): FOUND
- commit 28c27fd (feat: postinstall + bin/run.js): FOUND

---
*Phase: 08-npm-package*
*Completed: 2026-03-08*
