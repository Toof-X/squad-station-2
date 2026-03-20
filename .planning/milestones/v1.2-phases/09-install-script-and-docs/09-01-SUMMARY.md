---
phase: 09-install-script-and-docs
plan: "01"
subsystem: infra
tags: [install, shell, curl, posix-sh, github-releases, distribution]

requires:
  - phase: 07-ci-cd-pipeline
    provides: GitHub Releases binary assets named squad-station-{os}-{arch}

provides:
  - POSIX sh curl-pipe-sh installer script at repo root
  - Platform detection (darwin/linux, arm64/x86_64) via uname
  - Safe tempfile handling via mktemp and trap EXIT
  - /usr/local/bin install with ~/.local/bin fallback
  - Post-install binary executable verification

affects:
  - 09-02 (README documentation will reference install.sh)

tech-stack:
  added: []
  patterns:
    - "POSIX sh set -e for strict error propagation"
    - "mktemp + trap EXIT for safe temp file cleanup"
    - "uname -s | tr lower + case for portable OS detection"
    - "Writable test [ -w dir ] with fallback to user-local bin"

key-files:
  created:
    - install.sh
  modified: []

key-decisions:
  - "BASE_URL uses literal thientranhung/squad-station string for grep-pattern verifiability"
  - "ARCH normalized to arm64 for both arm64 and aarch64 uname outputs"
  - "FALLBACK flag tracks whether ~/.local/bin was used to conditionally print PATH advice"
  - "curl uses --proto '=https' --tlsv1.2 for TLS security"

patterns-established:
  - "Install script pattern: detect -> resolve dir -> mktemp/trap -> download -> mv+chmod -> verify executable"

requirements-completed:
  - INST-01
  - INST-02
  - INST-03

duration: 2min
completed: "2026-03-08"
---

# Phase 9 Plan 01: Install Script Summary

**POSIX sh curl-pipe-sh installer that detects OS/arch via uname, downloads the correct GitHub Releases binary, and installs to /usr/local/bin with ~/.local/bin fallback**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-08T17:05:34Z
- **Completed:** 2026-03-08T17:06:41Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Created `install.sh` as a POSIX sh script (`#!/bin/sh`, `set -e`) that works without Node.js
- Detects OS (darwin/linux) and arch (arm64/x86_64) with clear exit 1 + manual download URL on unsupported platforms
- Downloads the correct `squad-station-${OS}-${ARCH}` asset from GitHub Releases v0.1.0 using curl with `--proto '=https' --tlsv1.2`
- Safe temp file handling with `mktemp` and `trap 'rm -f "$TMPFILE"' EXIT`
- Installs to `/usr/local/bin`; falls back to `~/.local/bin` (auto-created) if not writable, with PATH advice printed
- Verifies binary is executable post-install; exits 1 with error if not

## Task Commits

Each task was committed atomically:

1. **Task 1: Write install.sh** - `0847c97` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `install.sh` — POSIX sh curl-pipe-sh installer for squad-station from GitHub Releases

## Decisions Made

- `BASE_URL` uses the literal `thientranhung/squad-station` string instead of variable interpolation so the grep verification pattern `github.com/thientranhung/squad-station/releases/download` matches directly in the file.
- Both `arm64` and `aarch64` uname outputs are normalized to `arm64` to match the Phase 7 binary naming convention.
- `FALLBACK` integer flag used instead of string comparison for portability across POSIX shells.
- curl flags include `--proto '=https' --tlsv1.2` for secure-by-default downloads.

## Deviations from Plan

None — plan executed exactly as written. One auto-fix applied: initial URL construction used `${REPO}` variable interpolation which caused the grep verification check (`github.com/thientranhung/squad-station/releases/download`) to fail on literal match. Fixed by extracting `BASE_URL` as a literal constant containing the full repository path.

## Issues Encountered

The grep verification check in the plan requires the literal string `github.com/thientranhung/squad-station/releases/download` to appear in the file. Initial implementation used `${REPO}` variable, so the literal wasn't present. Fixed by adding `BASE_URL` constant with the full literal URL prefix.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `install.sh` is at repo root, passes POSIX sh syntax check, and is executable (0755)
- GitHub Releases URL is pinned to v0.1.0 — this will need updating when version bumps
- Ready for Phase 9 Plan 02: README documentation linking to this installer

---
*Phase: 09-install-script-and-docs*
*Completed: 2026-03-08*
