# Phase 17: Init Flow Integration - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Generate squad.yml from the WizardResult collected in Phase 16, then continue with agent registration. Handle re-init (squad.yml already exists) with a plain-terminal prompt offering overwrite, add agents, or abort. Both paths complete with a valid squad.yml on disk before agent registration begins.

File writing and re-init are exclusively Phase 17's responsibility. Phase 16 (TUI wizard) only collects values and returns them.

</domain>

<decisions>
## Implementation Decisions

### squad.yml generation
- Write YAML using manual `format!` string construction — no Serialize derive needed
- SDD field written as structured YAML matching existing `SddConfig` schema:
  ```yaml
  sdd:
    - name: get-shit-done
      playbook: ".squad/sdd/gsd-playbook.md"
  ```
  Path convention: `.squad/sdd/{sdd.as_str()}-playbook.md` (same directory as `.squad/station.db`)
- Model validation: update `valid_models_for` in `config.rs` to accept full model IDs (`"claude-sonnet-4-6"`, `"claude-opus-4-6"`, `"claude-haiku-4-5"`, etc.) alongside or replacing the current short names (`"sonnet"`, `"opus"`, `"haiku"`)
- Generated squad.yml is written to `config_path` (the path passed to `init::run`) before calling `load_config`

### Re-init prompt (INIT-05)
- Plain terminal prompt — no TUI. Print 3 choices to stdout, read a single keypress
- Prompt text (shown when squad.yml already exists at init time):
  ```
  squad.yml already exists. What would you like to do?
    [o] Overwrite — replace with new wizard config
    [a] Add agents — add more workers to existing config
    [q] Abort — exit without changes
  ```
- `o` → run full wizard, write result to squad.yml (replacing it entirely)
- `a` → run worker-only wizard, append new workers to existing squad.yml
- `q` → exit cleanly, squad.yml unchanged
- Any other key: re-display prompt
- Ctrl+C at any point: exit cleanly (same as abort)

### "Add agents" behavior
- Launch TUI wizard in worker-only mode: skip Project and OrchestratorConfig pages, jump directly to WorkerCount + WorkerConfig pages
- New workers are **appended** to the existing `agents` array in squad.yml — existing agents are preserved unchanged
- wizard.rs needs a new entry point: `run_worker_only() -> anyhow::Result<Option<Vec<AgentInput>>>` (or a parameter to `run()`)
- After appending, the updated squad.yml is written back before proceeding with init

### Overwrite behavior
- Full wizard from scratch — all pages including Project, SDD, Orchestrator, Workers
- Result replaces squad.yml entirely (same as first-time generation)

### init.rs flow after Phase 17
```
1. config_path.exists()?
   No  → run full wizard → generate squad.yml → fall through to load_config
   Yes → plain prompt (o/a/q)
     o → run full wizard → overwrite squad.yml → fall through to load_config
     a → run worker-only wizard → append workers to squad.yml → fall through to load_config
     q → print "Init aborted." → return Ok(())
2. load_config (existing flow — unchanged)
3. register agents, launch tmux, hooks, etc.
```

### squad.yml format to generate
```yaml
project: {project_name}

sdd:
  - name: {sdd_workflow}
    playbook: ".squad/sdd/{sdd_workflow}-playbook.md"

orchestrator:
  provider: {provider}
  # name: {name}  (only if non-empty)
  # model: {model}  (only if Some)
  # description: {desc}  (only if Some)

agents:
  - role: worker
    provider: {provider}
    # name: {name}  (only if non-empty)
    # model: {model}  (only if Some)
    # description: {desc}  (only if Some)
```

### Claude's Discretion
- Exact YAML indentation style (2 spaces standard)
- Whether to include commented-out optional fields or omit them entirely when None/empty
- Exact wording of terminal prompt and abort/success messages
- Whether `run_worker_only` is a separate function or a parameter to `run()`

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 17 requirements
- `.planning/REQUIREMENTS.md` — INIT-04, INIT-05 (the two requirements this phase completes)

### Phase 16 output (WizardResult API)
- `.planning/phases/16-tui-wizard/16-01-SUMMARY.md` — actual WizardResult, AgentInput types, SddWorkflow enum
- `.planning/phases/16-tui-wizard/16-02-PLAN.md` — interfaces section with updated WizardResult/AgentInput types

### Existing code to modify
- `src/commands/init.rs` — current guard clause (Phase 16 placeholder); Phase 17 replaces the print block
- `src/commands/wizard.rs` — needs `run_worker_only()` or equivalent for "add agents" path
- `src/config.rs` — `valid_models_for` needs updating for full model IDs; `SquadConfig`/`AgentConfig` struct shapes define what squad.yml must contain

No external specs — requirements fully captured in decisions above.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/commands/wizard.rs::run()` — full wizard entry point (Phase 16); Phase 17 calls this for first-init and overwrite paths
- `src/config.rs::load_config()` — already called after squad.yml is written; Phase 17 doesn't change this
- `src/config.rs::SquadConfig` / `AgentConfig` — struct shapes define the squad.yml schema to generate

### Established Patterns
- `init.rs` guard clause pattern (Phase 16): check file existence, branch, return early — Phase 17 extends this pattern
- `serde_saphyr` for YAML reading — the generated YAML must parse cleanly through this
- `AgentConfig` has `#[serde(deny_unknown_fields)]` — generated YAML must not include unknown fields

### Integration Points
- `init.rs::run()` is the sole integration point — wizard called from here, squad.yml written here, then existing `load_config` flow takes over unchanged
- `config.rs::valid_models_for` — update here to accept full model IDs from wizard
- `wizard.rs::run()` — add worker-only variant here; keep existing `run()` signature unchanged for compatibility

</code_context>

<specifics>
## Specific Ideas

- SDD playbook path convention: `.squad/sdd/{workflow}-playbook.md` (e.g. `.squad/sdd/get-shit-done-playbook.md`) — co-located with `.squad/station.db`
- The plain-text re-init prompt is intentional: TUI not needed for a 3-choice branch decision

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

*Phase: 17-init-flow-integration*
*Context gathered: 2026-03-17*
