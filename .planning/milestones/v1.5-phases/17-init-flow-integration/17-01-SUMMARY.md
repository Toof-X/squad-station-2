---
phase: 17-init-flow-integration
plan: "01"
subsystem: config-validation, init-flow, wizard
tags: [model-validation, yaml-generation, wizard, tdd]
dependency_graph:
  requires: [16-01, 16-02]
  provides: [squad.yml generation from WizardResult, full model ID validation, worker-only wizard entry]
  affects: [src/config.rs, src/commands/init.rs, src/commands/wizard.rs]
tech_stack:
  added: []
  patterns: [TDD red-green-refactor, YAML string building, Rust enum extension]
key_files:
  created: []
  modified:
    - src/config.rs
    - src/commands/init.rs
    - src/commands/wizard.rs
decisions:
  - "generate_squad_yml builds YAML as a String (not serde_yaml serialization) to keep field ordering deterministic and avoid extra dependencies"
  - "KeyAction::Cancel variant added to wizard to handle worker-only Esc cancellation cleanly without repurposing Continue"
  - "worker_only: bool on WizardState rather than passing a flag to handle_key — avoids changing 10+ call sites"
metrics:
  duration_minutes: 6
  completed_date: "2026-03-17"
  tasks_completed: 2
  files_modified: 3
---

# Phase 17 Plan 01: Init Flow Integration Summary

**One-liner:** Squad.yml generation from WizardResult with full model ID validation and a worker-only wizard entry point.

## What Was Built

After Phase 16 collected wizard answers, this plan wires the output into a working init flow:

1. **Model validation expanded** (`src/config.rs`): `valid_models_for` now accepts both legacy short names (`opus`, `sonnet`, `haiku`) and the full model IDs the wizard produces (`claude-sonnet-4-6`, `claude-opus-4-6`, `claude-haiku-4-5`, `gemini-2.5-pro`, `gemini-2.5-flash`, `gemini-2.5-flash-lite`).

2. **Squad.yml generation** (`src/commands/init.rs`): New `generate_squad_yml(&WizardResult) -> String` function produces a valid YAML file with `project`, `sdd`, `orchestrator`, and `agents` sections. Optional fields (empty name, None model, None description) are omitted. The init wizard branch now writes this YAML to disk and falls through to `load_config` instead of returning early with a placeholder message.

3. **Worker-only wizard entry** (`src/commands/wizard.rs`): New `pub async fn run_worker_only() -> anyhow::Result<Option<Vec<AgentInput>>>` starts at `WorkerCount` page, skips `Project` and `OrchestratorConfig`. Added `worker_only: bool` to `WizardState`, `KeyAction::Cancel` variant, and updated `WorkerCount` Esc handler to cancel when `worker_only` is true.

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Update model validation + add generate_squad_yml (TDD) | 289f3d2 | src/config.rs, src/commands/init.rs |
| 2 | Add worker-only wizard entry point | c0cfefb | src/commands/wizard.rs |

## Key Decisions

- **String building for YAML** — `generate_squad_yml` builds the YAML manually as a String rather than using a serialization library. This ensures deterministic field ordering and avoids adding a new `serde_yaml` dependency when the output structure is simple and well-defined.

- **`KeyAction::Cancel` variant** — Added a `Cancel` variant to `KeyAction` rather than repurposing `Continue` or adding an out-parameter. Makes the intent explicit and does not change any existing call sites.

- **`worker_only` field on `WizardState`** — A single boolean on the state struct is cleaner than threading a parameter through `handle_key` and all its callees.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written.

The only adaptation was recognizing that the plan's pseudo-code for `run_worker_only` referenced `restore_terminal()` as a standalone function, but the actual codebase has `restore_terminal(&mut terminal)`. The implementation matched the actual signature.

## Verification

- `cargo test` — 201 tests pass (0 failures), including 8 new tests
- `cargo check` — no compilation errors
- `cargo build --release` — release binary builds cleanly
- Generated YAML round-trips: `generate_squad_yml(result)` -> `serde_saphyr::from_str` -> `SquadConfig` with correct field values (verified by `test_generate_squad_yml_roundtrips_through_serde`)

## Self-Check: PASSED

All files verified:
- src/config.rs — FOUND (contains "claude-sonnet-4-6")
- src/commands/init.rs — FOUND (contains "fn generate_squad_yml")
- src/commands/wizard.rs — FOUND (contains "pub async fn run_worker_only" and "worker_only: bool")

All commits verified:
- 289f3d2 — FOUND
- c0cfefb — FOUND
