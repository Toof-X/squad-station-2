# Phase 10: Centralized Hooks - Research

**Researched:** 2026-03-09
**Domain:** Rust CLI argument parsing (clap 4), environment variable auto-detection, tmux pane-to-session mapping, shell hook registration
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| HOOK-01 | `signal` command accepts `$TMUX_PANE` env var to auto-detect agent session name (hook requires no args beyond env var) | Current signal.rs already reads `TMUX_PANE` for the guard. The same env var value can be used to derive the session name via `tmux list-panes`. The `agent` arg in `cli.rs` must become `Option<String>`. |
| HOOK-02 | `hooks/claude-code.sh` and `hooks/gemini-cli.sh` marked deprecated in file headers (kept as reference only) | File-header deprecation is a comment-only change. Both scripts remain functional; only a `# DEPRECATED` notice and migration instruction are added at the top. |

</phase_requirements>

---

## Summary

Phase 10 eliminates the wrapper shell scripts (`hooks/claude-code.sh`, `hooks/gemini-cli.sh`) as a requirement for provider hook registration. Instead of calling a shell script that calls the binary, users can register `squad-station signal $TMUX_PANE` directly in `settings.json` (Claude Code) or `.gemini/settings.json` (Gemini CLI) Stop/AfterAgent hooks. The binary itself then resolves the session name from the pane ID.

The core technical change is in two places. First, `src/cli.rs`: the `Signal` subcommand's `agent` argument must become optional (`Option<String>`). Second, `src/commands/signal.rs`: when `agent` is `None`, the command reads `$TMUX_PANE` from the environment and maps it to a session name using `tmux list-panes -t $TMUX_PANE -F '#S'` — the exact logic already present in both hook scripts.

HOOK-02 is a documentation change only: add deprecation headers to the two shell scripts with a note pointing to the inline command syntax.

**Primary recommendation:** Make `agent` optional in `cli.rs`, add pane-to-session resolution inside `signal.rs` when agent is `None`, add integration test for the env-var-only invocation path, then add deprecation headers to the two scripts.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap 4 | 4.5 (pinned in Cargo.toml) | CLI argument parsing | Already in project; `Option<String>` arg with no default is the standard pattern |
| std::env | stdlib | `TMUX_PANE` env var read | No dependency needed |
| std::process::Command | stdlib | `tmux list-panes` subprocess | Already used throughout `tmux.rs` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| anyhow | 1.0 | Error propagation | Already used everywhere; wrap tmux call failures |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `tmux list-panes` subprocess to map pane to session | Parse `$TMUX_PANE` as session index | `$TMUX_PANE` is `%N` (pane ID), not session name. Cannot derive session name without calling tmux. Subprocess is the only correct approach. |
| Optional positional arg | `--pane` flag | Positional is ergonomic for inline hook registration; `squad-station signal $TMUX_PANE` reads naturally. No flag needed. |

**Installation:** No new dependencies required.

---

## Architecture Patterns

### Current Signal Command Structure

```
src/cli.rs           — Signal { agent: String }        ← must become Option<String>
src/commands/signal.rs  — run(agent: String, json: bool) ← must become run(agent: Option<String>, ...)
hooks/claude-code.sh    — resolves session name, calls binary
hooks/gemini-cli.sh     — same pattern
```

### Target Structure After Phase 10

```
src/cli.rs           — Signal { agent: Option<String> }   (no default)
src/commands/signal.rs  — run(agent: Option<String>, json: bool)
                            if agent.is_none():
                              read TMUX_PANE from env
                              call tmux list-panes to get session name
                              proceed with session name as agent
hooks/claude-code.sh    — DEPRECATED header + inline command example (file kept)
hooks/gemini-cli.sh     — DEPRECATED header + inline command example (file kept)
```

### Pattern 1: Optional Positional Argument in clap 4

**What:** A positional argument that is optional. When absent, the command falls back to env-var resolution.
**When to use:** When the value can be derived from the environment automatically but callers may also supply it explicitly.

```rust
// Source: clap 4 derive API — #[arg] on Option<T> makes the arg optional
/// Signal agent completion
Signal {
    /// Agent name or tmux pane ID (e.g. %0). If omitted, reads $TMUX_PANE from environment.
    agent: Option<String>,
},
```

The `run` signature changes in lock-step:
```rust
// src/commands/signal.rs
pub async fn run(agent: Option<String>, json: bool) -> anyhow::Result<()> {
```

### Pattern 2: Pane-to-Session Name Resolution

**What:** Given `$TMUX_PANE` (value like `%3`), derive the tmux session name.
**Why:** Agent names equal session names. The pane ID alone is not sufficient.

```rust
// Source: same logic as existing hooks/claude-code.sh line 17
// tmux list-panes -t "$TMUX_PANE" -F '#S'
fn session_name_from_pane(pane_id: &str) -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args(["list-panes", "-t", pane_id, "-F", "#S"])
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)?;
    Some(name)
}
```

This function belongs in `src/tmux.rs` alongside existing `session_exists`, `list_live_session_names` etc.

### Pattern 3: Guard Ordering with Optional Agent

The existing guard chain in `signal.rs` must be extended. The new guard (pane resolution) is inserted between GUARD 1 (TMUX_PANE check) and GUARD 2 (config/DB):

```
GUARD 1: TMUX_PANE set?      → if not in tmux and agent is None, silent exit 0
GUARD 1b: resolve agent      → if agent is None, call tmux to get session name
                                if resolution fails, silent exit 0
GUARD 2: config/DB
GUARD 3: agent registered?
GUARD 4: orchestrator self-signal?
```

When `agent` is `Some(name)`, the existing flow is unchanged. This ensures backward compatibility — callers that still pass an explicit agent name continue to work.

### Pattern 4: Deprecation Header Format

```bash
#!/bin/bash
# DEPRECATED: This script is no longer required as of squad-station v1.3.
# Use inline hook command instead:
#
#   squad-station signal $TMUX_PANE
#
# Register this directly in your provider's settings.json (no wrapper script needed).
# This file is kept for reference only.
#
# hooks/claude-code.sh -- Signal squad-station when Claude Code finishes a response
# ...rest of existing header unchanged...
```

### Anti-Patterns to Avoid

- **Removing the `agent: String` positional entirely and mandating env-var only:** Breaks all existing callers and test fixtures that call `signal <name>` explicitly. Keep `Option<String>` not remove the argument.
- **Deleting the hook scripts:** HOOK-02 says mark deprecated, not delete. They serve as reference documentation for users still on old setup.
- **Resolving the pane inside `cli.rs` before dispatch:** Keep resolution inside `signal.rs`. The CLI layer should not touch environment semantics; that belongs in the command layer.
- **Calling tmux when agent is `Some`:** The resolution subprocess should only run when `agent.is_none()`. Never call `tmux list-panes` when an explicit name was given.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Pane-to-session mapping | Custom regex on `$TMUX_PANE` | `tmux list-panes -t $PANE -F '#S'` | Pane ID format is an implementation detail of tmux; `-F '#S'` is the canonical way to get session name |
| Optional arg fallback | Custom argument parser | clap `Option<String>` on positional arg | clap 4 handles optional positional correctly with no boilerplate |

**Key insight:** The hook scripts already contain the correct tmux invocation. Translating those ~4 shell lines into a Rust function in `tmux.rs` is the entire implementation of HOOK-01's pane-resolution side.

---

## Common Pitfalls

### Pitfall 1: `#S` is a tmux format string, not a shell variable

**What goes wrong:** Escaping `#S` as `\#S` or `'#S'` in a Rust string literal passed to `Command::arg()` causes tmux to receive a backslash-prefixed string and return nothing.
**Why it happens:** In Rust `Command::args()`, each argument is passed as-is (no shell interpolation). `#S` does not need escaping — it is the literal tmux format specifier.
**How to avoid:** Use `"#S"` as a plain `&str` in the `args` array. The existing hook script uses `'#S'` (single-quoted in shell to prevent `$` expansion) but in Rust there is no shell involved.
**Warning signs:** `session_name_from_pane` returns `None` even when pane exists.

### Pitfall 2: `tmux list-panes -t %N` may fail when not inside a tmux server

**What goes wrong:** If tmux is not running at all, `Command::new("tmux").args([...])` returns a non-zero exit code. `output().ok()?` silently returns `None`, which then triggers silent exit 0 (correct behavior) — but only if `None` is handled as a guard, not an error.
**Why it happens:** The binary may be invoked in CI/CD or directly from a shell without tmux. GUARD 1 (`TMUX_PANE` env check) catches this case before resolution is attempted, but only when `agent.is_none()`. If `agent.is_some()` and `TMUX_PANE` is set by some other means, resolution is skipped anyway.
**How to avoid:** Resolution only runs when `agent.is_none()` AND `TMUX_PANE` is set (GUARD 1 already confirmed tmux context). The guard ordering ensures this.

### Pitfall 3: clap optional positional interacts with `--help` display

**What goes wrong:** `Option<String>` positional args show as `[AGENT]` in help text. The help text for the `Signal` subcommand must clarify that the pane ID is also accepted.
**Why it happens:** clap generates `[AGENT]` which suggests any string — not obviously a pane ID.
**How to avoid:** Update the `#[arg(help = "...")]` doc string to explain both forms: `signal my-agent` and `signal $TMUX_PANE` (which expands to e.g. `signal %3`).

### Pitfall 4: Existing tests pass `"agent"` as `String` — signature change breaks compilation

**What goes wrong:** `commands::signal::run(agent, json)` is called with `agent: String` in `main.rs`. After the change to `Option<String>`, the call site in `main.rs` and all test call sites must wrap/unwrap accordingly.
**Why it happens:** Rust will not compile if the type signature changes without updating callers.
**How to avoid:** In `main.rs` the dispatch is `Signal { agent } => commands::signal::run(agent, cli.json).await` — after the change, `agent` is already `Option<String>` from clap, no wrapping needed. Test fixtures that call the DB directly (not through `signal::run`) are unaffected.

---

## Code Examples

### signal.rs — modified run signature and new guard

```rust
// Adapted from existing signal.rs pattern
pub async fn run(agent: Option<String>, json: bool) -> anyhow::Result<()> {
    // GUARD 1: Not in tmux and no explicit agent -- silent exit 0
    let tmux_pane = std::env::var("TMUX_PANE").ok();
    if agent.is_none() && tmux_pane.is_none() {
        return Ok(());
    }

    // GUARD 1b: resolve agent name when not supplied explicitly
    let agent: String = match agent {
        Some(name) => name,
        None => {
            // tmux_pane is Some here (guard above ensures at least one is present)
            let pane = tmux_pane.unwrap();
            match tmux::session_name_from_pane(&pane) {
                Some(name) => name,
                None => return Ok(()), // cannot determine session -- silent exit 0
            }
        }
    };

    // --- remainder of existing guard chain (GUARD 2, 3, 4) unchanged ---
    // ...
}
```

### tmux.rs — new helper function

```rust
/// Resolve tmux session name from a pane ID (e.g. "%3" → "my-agent").
/// Returns None if tmux is not running or pane ID is invalid.
pub fn session_name_from_pane(pane_id: &str) -> Option<String> {
    let output = std::process::Command::new("tmux")
        .args(["list-panes", "-t", pane_id, "-F", "#S"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}
```

### cli.rs — updated Signal variant

```rust
/// Signal agent completion
Signal {
    /// Agent name or tmux pane ID (e.g. %3). Omit to auto-detect from $TMUX_PANE.
    agent: Option<String>,
},
```

### main.rs — dispatch (no change needed)

```rust
// clap derives Option<String> for agent -- passes through unchanged
Signal { agent } => commands::signal::run(agent, cli.json).await,
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Shell wrapper resolves pane → session, calls binary | Binary resolves pane → session internally | Phase 10 (v1.3) | Zero-script hook registration; one fewer file to maintain per provider |
| `signal <agent-name>` required | `signal` or `signal $TMUX_PANE` or `signal <agent-name>` | Phase 10 | Backward compatible — all three forms work |

**Deprecated/outdated:**
- `hooks/claude-code.sh` and `hooks/gemini-cli.sh` as required hook scripts: replaced by inline `squad-station signal $TMUX_PANE`. Kept as reference.

---

## Open Questions

1. **Should `signal` with explicit `$TMUX_PANE` value (e.g. `signal %3`) be supported, or only zero-arg resolution?**
   - What we know: HOOK-01 says "auto-detect agent session name" with `$TMUX_PANE` set in the environment. The success criterion says `squad-station signal $TMUX_PANE` can be placed directly in hooks (where `$TMUX_PANE` expands at registration time to the pane ID like `%3`).
   - What's unclear: Whether the binary should treat any argument starting with `%` as a pane ID and resolve it, vs. treat all explicit arguments as literal agent names.
   - Recommendation: Treat `agent: Option<String>` as `None` only when omitted entirely. When provided (even if it looks like `%3`), treat as an explicit agent name — because `$TMUX_PANE` expands before the binary runs, so the binary receives the resolved string. This is the simplest and most predictable behavior.

   **Confirmed reading of success criterion:** "The inline hook command `squad-station signal $TMUX_PANE` can be placed directly in settings.json" — this means the shell expands `$TMUX_PANE` at invocation time, so the binary receives `%3` (the pane ID) as the `agent` argument. Therefore, the binary MUST handle the case where `agent = Some("%3")` and resolve it to a session name. This is different from `agent = None`.

   **Revised approach:** When `agent` is `Some(s)` and `s` starts with `%`, treat it as a pane ID and resolve via `tmux list-panes`. When `agent` is `None`, read `$TMUX_PANE` from env and resolve. Both paths call `session_name_from_pane`.

---

## Validation Architecture

nyquist_validation is enabled (config.json).

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + tokio-test (async) |
| Config file | Cargo.toml `[dev-dependencies]` — no separate config file |
| Quick run command | `cargo test signal` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HOOK-01 | `signal` with `TMUX_PANE` set and no agent arg resolves session name and signals | integration | `cargo test test_signal_via_tmux_pane` | ❌ Wave 0 |
| HOOK-01 | `signal %3` (pane ID as explicit arg) resolves session name | integration | `cargo test test_signal_pane_id_as_arg` | ❌ Wave 0 |
| HOOK-01 | `signal` with no TMUX_PANE and no arg exits 0 silently | integration | `cargo test test_signal_no_args_no_tmux` | ❌ Wave 0 |
| HOOK-01 | `signal my-agent` (explicit name) still works unchanged | integration | `cargo test test_signal_explicit_agent_still_works` | ✅ `test_signal_orchestrator_self_signal_guard` covers the guard path; explicit name path is already tested |
| HOOK-01 | `session_name_from_pane` unit test (arg building) | unit | `cargo test test_session_name_from_pane_args` | ❌ Wave 0 |
| HOOK-02 | `hooks/claude-code.sh` has DEPRECATED header | manual / file check | `grep -q DEPRECATED hooks/claude-code.sh` | ❌ Wave 0 (file edit) |
| HOOK-02 | `hooks/gemini-cli.sh` has DEPRECATED header | manual / file check | `grep -q DEPRECATED hooks/gemini-cli.sh` | ❌ Wave 0 (file edit) |

### Sampling Rate
- **Per task commit:** `cargo test signal`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green (`cargo test`) before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `tests/test_integration.rs` — add `test_signal_via_tmux_pane`, `test_signal_pane_id_as_arg`, `test_signal_no_args_no_tmux` (note: tests that invoke pane-to-session resolution require an actual tmux session; these tests should mock or skip gracefully when tmux is not running, following the existing pattern in `test_send_no_tmux_session`)
- [ ] `src/tmux.rs` — add `test_session_name_from_pane_args` unit test (arg array verification, same pattern as `test_send_keys_args_have_literal_flag` — no actual tmux needed)

*(No new framework install needed — existing `cargo test` infrastructure covers all phase requirements)*

---

## Sources

### Primary (HIGH confidence)
- Direct code reading: `src/commands/signal.rs` — full guard chain and current `agent: String` flow
- Direct code reading: `src/cli.rs` — current `Signal { agent: String }` definition
- Direct code reading: `src/tmux.rs` — existing argument builder pattern (`send_keys_args`, `launch_args` etc.) confirms the style for `session_name_from_pane`
- Direct code reading: `hooks/claude-code.sh` line 17 — `tmux list-panes -t "$TMUX_PANE" -F '#S'` is the exact invocation to port to Rust
- Direct code reading: `tests/test_integration.rs` — `test_signal_*` tests confirm existing test patterns and what is already covered

### Secondary (MEDIUM confidence)
- clap 4 derive docs (training knowledge, version 4.5 confirmed in Cargo.toml): `Option<String>` on a positional arg makes it optional, rendered as `[AGENT]` in help

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in Cargo.toml, no new dependencies
- Architecture: HIGH — implementation is a direct Rust translation of existing shell script logic (4 lines of shell → ~10 lines of Rust), plus a clap type change
- Pitfalls: HIGH — the `#S` escaping pitfall and clap signature change are verifiable from code; tmux absence case is verified by existing guard logic
- Test gaps: HIGH — existing test file structure is clear; new tests follow identical patterns to `test_signal_orchestrator_self_signal_guard`

**Research date:** 2026-03-09
**Valid until:** 2026-06-09 (stable domain — tmux API and clap 4 are not fast-moving)
