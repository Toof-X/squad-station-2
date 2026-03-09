# Phase 11: Antigravity Provider Core - Research

**Researched:** 2026-03-09
**Domain:** Rust config parsing, provider-conditional behavior, integration testing patterns
**Confidence:** HIGH

## Summary

Phase 11 adds `antigravity` as a recognized orchestrator provider that operates in DB-only mode: no tmux session is created at init time, and no `tmux send-keys` notification is sent when workers signal completion. The changes are narrow and surgical — all three requirements (AGNT-01, AGNT-02, AGNT-03) touch existing files with small, isolated modifications.

The codebase already has the right seams. `config.rs` holds `AgentConfig.tool: String` with no validation — accepting "antigravity" costs zero change to parsing. `init.rs` already checks `tmux::session_exists` before launching; adding a provider check before that block is a single `if` guard. `signal.rs` already has a guard path that skips tmux (`session_exists` returns false → `orchestrator_notified = false`); adding an explicit provider check for `antigravity` keeps this behaviour unconditional rather than contingent on tmux state.

**Primary recommendation:** Add a helper method `AgentConfig::is_db_only() -> bool` that returns `true` when `tool == "antigravity"`. Use it in `init.rs` to skip `launch_agent` and in `signal.rs` to skip `send_keys_literal`. Write integration tests using the existing `test_config.rs` pattern and the `setup_file_db`/`write_squad_yml` pattern from `test_integration.rs`.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| AGNT-01 | `config.rs` supports `provider: antigravity` as valid orchestrator provider value with integration tests | Serde `String` field requires zero code change to parse; tests use `serde_saphyr::from_str` directly — test file is `tests/test_config.rs` |
| AGNT-02 | `signal.rs` skips `tmux send-keys` notification when orchestrator provider is `antigravity` | Need to expose orchestrator tool from `get_orchestrator()` result (`Agent.tool` already present); add conditional guard before `send_keys_literal` call in `signal.rs` lines 104-105 |
| AGNT-03 | `init.rs` skips tmux session creation for `antigravity` orchestrator — DB-only registration with clear log message | Add `is_db_only()` check before `tmux::launch_agent` on lines 36-41; update skipped count + log message; adjust "all failed" exit logic to not count db-only agents as failures |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde / serde_saphyr | existing | YAML → struct deserialization | Already in use; `tool: String` accepts any value without code change |
| sqlx | existing | SQLite async pool | Already in use for all DB ops |
| tokio | existing | Async runtime | Already in use for all command tests |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | existing | Isolated test DBs | Every integration test that needs a real SQLite file |
| serde_json | existing | JSON output in integration test assertions | Tests that verify `--json` output format |

**Installation:** No new dependencies required.

## Architecture Patterns

### Recommended Project Structure

No new files needed. Changes are in:
```
src/
├── config.rs          # Add is_db_only() helper on AgentConfig
├── commands/
│   ├── init.rs        # Skip tmux launch for antigravity orchestrator
│   └── signal.rs      # Skip send_keys_literal for antigravity orchestrator
tests/
├── test_config.rs     # Add AGNT-01 config parsing tests
└── test_integration.rs # Add AGNT-02 and AGNT-03 integration tests
```

### Pattern 1: Provider-Conditional Skip in init.rs

**What:** Check `config.orchestrator.is_db_only()` before calling `tmux::launch_agent`. If true, register to DB only and add the orchestrator to `skipped_names` with a distinct log message.

**When to use:** Whenever a provider cannot or should not have a tmux session.

**Example:**
```rust
// In init.rs, replace the orchestrator launch block (lines 36-41):
let orch_launched = if config.orchestrator.is_db_only() {
    // Antigravity: DB-only registration, no tmux session
    println!("  {} (db-only, no tmux session)", orch_name);
    false
} else if tmux::session_exists(&orch_name) {
    false
} else {
    tmux::launch_agent(&orch_name, &config.orchestrator.tool)?;
    true
};
```

### Pattern 2: Provider-Conditional Skip in signal.rs

**What:** After fetching the orchestrator record, check `orch.tool == "antigravity"` (or via helper) before calling `send_keys_literal`. The DB update (`update_status`) still happens — only the tmux notification is skipped.

**When to use:** When the orchestrator is polled rather than pushed-to.

**Example:**
```rust
// In signal.rs, inside the `if let Some(orch) = orchestrator` block:
if orch.is_db_only() {
    // Antigravity: orchestrator polls DB, no push notification needed
    false
} else if tmux::session_exists(&orch.name) {
    tmux::send_keys_literal(&orch.name, &notification)?;
    true
} else {
    false
}
```

Note: `Agent` (DB struct in `db/agents.rs`) needs a matching `is_db_only()` method, or the check can be inline: `orch.tool == "antigravity"`.

### Pattern 3: Config Helper Method

**What:** Add `is_db_only()` to `AgentConfig` and (optionally mirrored) to `Agent` for signal.rs.

**Example:**
```rust
// In config.rs, add to AgentConfig impl:
impl AgentConfig {
    pub fn is_db_only(&self) -> bool {
        self.tool == "antigravity"
    }
}
```

For `signal.rs` which works with the `Agent` DB struct, a simple inline `orch.tool == "antigravity"` is equally clear and avoids coupling `db/agents.rs` to config domain knowledge.

### Pattern 4: Integration Test for AGNT-01 (Config Parsing)

Follow `tests/test_config.rs` pattern exactly — `serde_saphyr::from_str` unit test, no DB needed:

```rust
#[test]
fn test_antigravity_tool_parses() {
    let yaml = "project: p\norchestrator:\n  tool: antigravity\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(cfg.orchestrator.tool, "antigravity");
}

#[test]
fn test_is_db_only_antigravity() {
    let yaml = "project: p\norchestrator:\n  tool: antigravity\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert!(cfg.orchestrator.is_db_only());
}

#[test]
fn test_is_db_only_claude_code_false() {
    let yaml = "project: p\norchestrator:\n  tool: claude-code\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert!(!cfg.orchestrator.is_db_only());
}
```

### Pattern 5: Integration Test for AGNT-02 and AGNT-03

Follow `test_signal_orchestrator_self_signal_guard` and `write_squad_yml` patterns from `test_integration.rs`. Write a helper `write_antigravity_squad_yml` that sets `tool: antigravity` for the orchestrator. Use `setup_file_db` + binary invocation via `cmd_with_db`.

For AGNT-03 (init skips tmux): use `--json` output and verify `launched` count is 0 for the orchestrator, and stdout contains the DB-only log message.

For AGNT-02 (signal skips tmux notify): register an antigravity orchestrator + worker in DB, signal the worker, verify message is `completed` in DB and agent status is `idle` — no tmux assertions needed since tmux is unavailable in test env.

### Anti-Patterns to Avoid

- **String comparison at every callsite:** Inline `tool == "antigravity"` checks scattered through multiple files. Use the `is_db_only()` helper on `AgentConfig` to keep the canonical definition in one place.
- **Blocking init entirely for antigravity orchestrator:** Only the tmux launch is skipped. DB registration (`insert_agent`) still runs — the orchestrator must exist in DB for signal routing.
- **Failing init when all agents are tmux-based but orchestrator is db-only:** The "all failed" exit logic (`failed.len() == total`) must not count the db-only orchestrator as a failed launch. The orchestrator is not "failed" — it intentionally has no session.
- **Modifying the `Agent` struct or DB schema:** No schema change needed. `tool` column already stores the provider string.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Provider type safety | Custom `Provider` enum with exhaustive match | Simple `is_db_only()` string helper | Enum would require updating every match site when new providers are added; string check is extensible without churn |
| Config validation | Explicit allowlist of valid tool values | Accept any `String`, gate behavior on known values | Validation would break forward compatibility; unknown providers should work as normal tmux providers |

**Key insight:** The `tool` field is deliberately untyped. Adding validation that rejects unknown values would be a regression — users with custom provider strings (e.g. internal tools) must not be broken. Gating DB-only behavior on a known value (`"antigravity"`) is additive, not restrictive.

## Common Pitfalls

### Pitfall 1: Counting db-only orchestrator as a failed launch

**What goes wrong:** After phase change, `orch_launched = false` and `orch_skipped` becomes `true` (same path as already-running sessions). But the "all failed" check at end of `init.rs` uses `failed.len() == total`. If the db-only orchestrator is added to `failed`, init exits with error when no real failure occurred.

**Why it happens:** The existing code increments `skipped` for both "already running" and (proposed) "db-only" paths. As long as db-only orchestrators do NOT get added to `failed`, the exit logic is fine.

**How to avoid:** Use `skipped` counter (not `failed`) for db-only path. Verify the "all failed" guard still works correctly: it should only error when every agent (including workers) actually errored.

**Warning signs:** Integration test for AGNT-03 fails with non-zero exit code when only the orchestrator is antigravity.

### Pitfall 2: Skipping DB registration for antigravity orchestrator

**What goes wrong:** If `is_db_only()` is checked before `insert_agent` (rather than before `launch_agent`), the orchestrator never appears in DB. Signal routing in `signal.rs` calls `get_orchestrator()` — if orchestrator is absent, notification is silently skipped. But this also means `list agents` won't show the orchestrator, breaking observability.

**Why it happens:** Misreading the requirement: "DB-only registration" means register IN DB but skip tmux, not skip DB.

**How to avoid:** Always call `insert_agent` for all agents regardless of provider. Only gate `launch_agent` on `is_db_only()`.

### Pitfall 3: Signal still notifies via tmux when orchestrator session happens to exist

**What goes wrong:** Current `signal.rs` guard is: `if tmux::session_exists(&orch.name)` → send keys. If someone manually creates a tmux session with the same name as the antigravity orchestrator, the code would send keys to it anyway, breaking the DB-only contract.

**Why it happens:** Relying on "session doesn't exist" as a proxy for "don't notify" rather than checking provider intent.

**How to avoid:** Check `is_db_only()` BEFORE `tmux::session_exists` in signal.rs. If db-only, skip tmux regardless of whether a session happens to exist.

### Pitfall 4: Log message wording for AGNT-03

**What goes wrong:** Vague or missing log message when init skips tmux for antigravity. Success criterion explicitly requires "a clear log message explaining DB-only registration."

**How to avoid:** Print something like: `"  {name}: db-only (antigravity orchestrator — no tmux session)"` or similar. Make it distinct from the "already running (skipped)" message so users can tell the difference.

## Code Examples

### Current signal.rs orchestrator notification block (lines 97-116)

```rust
// Source: src/commands/signal.rs
let orchestrator_notified = if rows > 0 {
    let orchestrator = db::agents::get_orchestrator(&pool).await?;
    if let Some(orch) = orchestrator {
        let task_id_str = task_id.as_deref().unwrap_or("unknown");
        let notification = format!("{} completed {}", agent, task_id_str);
        if tmux::session_exists(&orch.name) {
            tmux::send_keys_literal(&orch.name, &notification)?;
            true
        } else {
            false
        }
    } else {
        false
    }
} else {
    false
};
```

Modification: add `orch.tool == "antigravity"` check before `tmux::session_exists`.

### Current init.rs orchestrator launch block (lines 36-41)

```rust
// Source: src/commands/init.rs
let orch_launched = if tmux::session_exists(&orch_name) {
    false
} else {
    tmux::launch_agent(&orch_name, &config.orchestrator.tool)?;
    true
};
```

Modification: add `is_db_only()` check as first condition.

### Existing test_config.rs pattern (reference)

```rust
// Source: tests/test_config.rs
#[test]
fn test_project_is_string() {
    let yaml = "project: myapp\norchestrator:\n  tool: claude-code\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(cfg.project, "myapp");
}
```

AGNT-01 tests follow this exact pattern — same file, same function style.

### Existing write_squad_yml pattern (reference for AGNT-02/AGNT-03 integration tests)

```rust
// Source: tests/test_integration.rs
fn write_squad_yml(dir: &std::path::Path, _db_file: &std::path::Path) {
    let yaml = r#"project: test-squad
orchestrator:
  name: test-orch
  tool: claude-code
  role: orchestrator
agents: []
"#;
    std::fs::write(dir.join("squad.yml"), yaml).expect("failed to write squad.yml");
}
```

For AGNT-02/AGNT-03 tests, add a parallel `write_antigravity_squad_yml` with `tool: antigravity`.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Provider stored as `provider` column | Renamed to `tool` column | Phase 4/5 (v1.1) | Use `agent.tool` not `agent.provider` everywhere |
| `command` field in config | Removed (CONF-03) | Phase 4/5 (v1.1) | No `command` in YAML; `tool` infers launch command |

**Deprecated/outdated:**
- `provider` field in config: replaced by `tool` field — do not use `provider` as a YAML key in any new tests or docs.

## Open Questions

1. **What happens when all agents (workers too) are antigravity?**
   - What we know: The "all failed" guard in init.rs checks `failed.len() == total`. With db-only orchestrator and normal workers, this works fine.
   - What's unclear: If ALL agents (including workers) somehow set `is_db_only()`, init would exit successfully with 0 launched — probably fine but untested.
   - Recommendation: Scope the db-only check to orchestrator only per requirements. Worker agents are always tmux-based in v1.3.

2. **Should `is_db_only()` live on `AgentConfig` only, or also on the `Agent` DB struct?**
   - What we know: `signal.rs` works with `Agent` (DB struct), not `AgentConfig`. Both have a `tool` field.
   - What's unclear: Whether to add a parallel method to `Agent` or use inline comparison.
   - Recommendation: Use inline `orch.tool == "antigravity"` in signal.rs. Avoids coupling the DB struct to config semantics. If the same check is needed in 3+ places, promote to a module-level const or free function.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test + tokio-test (via `#[tokio::test]`) |
| Config file | `Cargo.toml` (no separate config file) |
| Quick run command | `cargo test test_antigravity` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AGNT-01 | `provider: antigravity` parses without error | unit | `cargo test test_antigravity_tool_parses` | ❌ Wave 0 |
| AGNT-01 | `is_db_only()` returns true for antigravity | unit | `cargo test test_is_db_only_antigravity` | ❌ Wave 0 |
| AGNT-01 | `is_db_only()` returns false for claude-code | unit | `cargo test test_is_db_only_claude_code_false` | ❌ Wave 0 |
| AGNT-02 | Signal skips tmux notify when orchestrator is antigravity | integration | `cargo test test_signal_antigravity_orchestrator_db_only` | ❌ Wave 0 |
| AGNT-02 | DB message still completed on antigravity signal | integration | `cargo test test_signal_antigravity_message_completed` | ❌ Wave 0 |
| AGNT-03 | init skips tmux launch for antigravity orchestrator | integration | `cargo test test_init_antigravity_orchestrator_skips_tmux` | ❌ Wave 0 |
| AGNT-03 | init prints db-only log message for antigravity | integration | `cargo test test_init_antigravity_log_message` | ❌ Wave 0 |
| AGNT-03 | init registers orchestrator in DB even when db-only | integration | `cargo test test_init_antigravity_registers_in_db` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test test_antigravity`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] Tests in `tests/test_config.rs` — covers AGNT-01 (config parsing + `is_db_only()`)
- [ ] Tests in `tests/test_integration.rs` — covers AGNT-02 and AGNT-03 (binary-level behavior)
- [ ] No new test framework needed — existing tokio + tempfile infrastructure covers all tests

## Sources

### Primary (HIGH confidence)

- Direct source read: `src/config.rs` — `AgentConfig.tool: String`, no validation, accepts any string
- Direct source read: `src/commands/init.rs` — orchestrator launch flow on lines 36-41, skipped counter logic
- Direct source read: `src/commands/signal.rs` — orchestrator notification block lines 97-116, `tmux::session_exists` guard
- Direct source read: `src/db/agents.rs` — `Agent.tool` field present, `get_orchestrator()` returns full `Agent` struct
- Direct source read: `tests/test_config.rs` — test pattern for config parsing
- Direct source read: `tests/test_integration.rs` — `write_squad_yml`, `setup_file_db`, `cmd_with_db` patterns
- Direct source read: `tests/helpers.rs` — `setup_test_db()` for unit-style async tests

### Secondary (MEDIUM confidence)

- `.planning/REQUIREMENTS.md` — AGNT-01/02/03 definitions
- `.planning/ROADMAP.md` — Phase 11 success criteria
- `.planning/STATE.md` — v1.3 context decisions ("Antigravity provider = DB-only orchestrator")

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries are already in use; no new deps
- Architecture: HIGH — all seams identified by reading actual source files; changes are localized and follow existing patterns
- Pitfalls: HIGH — derived from reading actual code paths (failed count logic, tmux guard ordering, DB registration requirement)

**Research date:** 2026-03-09
**Valid until:** 2026-04-09 (stable Rust project; only invalidated if init.rs or signal.rs is significantly refactored)
