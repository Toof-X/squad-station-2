# Phase 13: Safe Injection & Documentation - Research

**Researched:** 2026-03-09
**Domain:** Rust tmux adapter (load-buffer/paste-buffer), PLAYBOOK documentation
**Confidence:** HIGH

---

## Summary

Phase 13 has two independent workstreams: (1) safe multiline tmux injection via `load-buffer`/`paste-buffer`, and (2) a complete PLAYBOOK.md rewrite. Both workstreams are well-specified in the existing design documents and accumulated project decisions. No external library research is needed — the implementation uses only `std::process::Command` and `std::fs` (already used throughout the codebase).

The safe injection work replaces `send_keys_literal` in `send.rs` with a new `tmux.rs` function (`inject_body`) that writes content to a temp file, calls `tmux load-buffer <path>`, calls `tmux paste-buffer -t <target>`, and cleans up the temp file. All arg-builder functions in `tmux.rs` follow a testable pattern (pure functions returning `Vec<String>`) that must be applied to the new functions.

The PLAYBOOK rewrite must document: (a) `squad-station signal $TMUX_PANE` as the canonical inline hook (replacing shell script references), (b) Antigravity provider with exact `squad.yml` syntax, and (c) notification hook registration (GAP-04 deferred, now addressed by DOCS-03).

**Primary recommendation:** Implement `inject_body` in `tmux.rs` following the existing arg-builder pattern, update `send.rs` to call it, then write PLAYBOOK.md from the design docs as the authoritative reference.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TMUX-01 | `tmux.rs` implements `load-buffer`/`paste-buffer` pattern for safe multiline injection via temp file + cleanup | Confirmed: tmux load-buffer + paste-buffer is the standard pattern for safe content delivery. Design spec in docs/SOLUTION-DESIGN.md §6.5. Implementation uses `std::process::Command` + `std::fs::write` + `tempfile` crate (already a dev-dep, need to add as regular dep or use `std::env::temp_dir`). |
| TMUX-02 | `send` command uses safe tmux adapter for all body content delivery (replaces direct `send-keys` for content) | Confirmed: `send.rs` line 58 currently calls `tmux::send_keys_literal(&agent, &body)`. Replace this single call with new `tmux::inject_body(&agent, &body)`. |
| DOCS-01 | `PLAYBOOK.md` rewritten with centralized hook setup documenting `squad-station signal $TMUX_PANE` inline command | Confirmed: hooks/claude-code.sh already shows the deprecated form and correct inline form. All hook setup details are in docs/SOLUTION-DESIGN.md §6.1 and §6.4. |
| DOCS-02 | `PLAYBOOK.md` documents Antigravity provider and IDE orchestrator mode | Confirmed: full Antigravity design in docs/SOLUTION-DESIGN.md §2 and squad.yml comment block. `provider: antigravity` is the exact YAML key. |
| DOCS-03 | `PLAYBOOK.md` covers notification hook registration (deferred since GAP-04) | Confirmed: hooks/claude-code-notify.sh and hooks/gemini-cli-notify.sh exist. GAP-04 notes the Notification event uses `permission_prompt` matcher for Claude Code, plain `Notification` for Gemini CLI. |
</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `std::process::Command` | stdlib | Run tmux subcommands | Already used for all tmux calls in `tmux.rs` |
| `std::fs` | stdlib | Write temp file for buffer | Already used throughout codebase |
| `std::env::temp_dir()` | stdlib | Get OS temp directory | Avoids adding `tempfile` as a runtime dep |
| `uuid` | 1.8 | Unique temp file name | Already in `Cargo.toml` — use `Uuid::new_v4()` |
| `anyhow` | 1.0 | Error propagation | Already used in all commands |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tempfile` | 3 | Temp file management | Already a dev-dependency; NOT needed for runtime — use `std::env::temp_dir()` instead |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `std::env::temp_dir() + uuid` | `tempfile` crate | `tempfile` auto-cleans on drop, but requires adding runtime dep; manual cleanup keeps deps minimal and matches project style |
| `tmux paste-buffer` | `tmux send-keys -l` | `send-keys -l` breaks on multiline and very long content; `paste-buffer` handles arbitrary content correctly |

**Installation:** No new runtime dependencies required.

---

## Architecture Patterns

### Recommended tmux.rs Structure

The existing `tmux.rs` uses a strict testable pattern:

1. Private arg-builder functions (pure, return `Vec<String>`) — these are unit-tested directly
2. Public API functions that call `Command::new("tmux").args(...)` using those builders

New functions must follow this exact same pattern.

### Pattern 1: Arg-Builder Functions (Existing Pattern)

**What:** Each tmux subcommand has a private function that returns `Vec<String>`. This is testable without spawning tmux.

**When to use:** Every new tmux subcommand in this phase.

**Example (existing):**
```rust
// Source: src/tmux.rs (existing codebase)
fn send_keys_args(target: &str, text: &str) -> Vec<String> {
    vec![
        "send-keys".to_string(),
        "-t".to_string(),
        target.to_string(),
        "-l".to_string(),
        text.to_string(),
    ]
}
```

**New arg-builders needed for TMUX-01:**
```rust
fn load_buffer_args(path: &str) -> Vec<String> {
    vec![
        "load-buffer".to_string(),
        path.to_string(),
    ]
}

fn paste_buffer_args(target: &str) -> Vec<String> {
    vec![
        "paste-buffer".to_string(),
        "-t".to_string(),
        target.to_string(),
    ]
}
```

### Pattern 2: inject_body Public API

**What:** Write content to a temp file, load it into tmux buffer, paste to target, clean up.

**When to use:** Called from `send.rs` for all body content delivery (TMUX-02).

```rust
// New public function in src/tmux.rs
pub fn inject_body(target: &str, body: &str) -> Result<()> {
    // Step 1: Write content to temp file
    let temp_path = std::env::temp_dir()
        .join(format!("squad-station-msg-{}", uuid::Uuid::new_v4()));
    std::fs::write(&temp_path, body)?;

    // Step 2: Load into tmux named buffer
    let path_str = temp_path.to_str()
        .ok_or_else(|| anyhow::anyhow!("temp path contains invalid UTF-8"))?;
    let load_args = load_buffer_args(path_str);
    let status = Command::new("tmux").args(&load_args).status()?;
    if !status.success() {
        let _ = std::fs::remove_file(&temp_path);
        bail!("tmux load-buffer failed for target: {}", target);
    }

    // Step 3: Paste buffer into target session
    let paste_args = paste_buffer_args(target);
    let status = Command::new("tmux").args(&paste_args).status()?;
    let _ = std::fs::remove_file(&temp_path); // always cleanup
    if !status.success() {
        bail!("tmux paste-buffer failed for target: {}", target);
    }

    Ok(())
}
```

### Pattern 3: send.rs Call-Site Change

**What:** Replace the single `send_keys_literal` call with `inject_body`.

**Current (src/commands/send.rs line 58):**
```rust
// Source: src/commands/send.rs
tmux::send_keys_literal(&agent, &body)?;
```

**Replacement:**
```rust
tmux::inject_body(&agent, &body)?;
```

No other changes needed in `send.rs`.

### Pattern 4: PLAYBOOK.md Document Structure

The PLAYBOOK does not exist yet (no PLAYBOOK.md in root). It must be created at `PLAYBOOK.md` in the repo root (README.md references `docs/PLAYBOOK.md` but docs/PLAYBOOK.md is the target path to check).

Key sections PLAYBOOK must cover:
1. **Prerequisites** — tmux, squad-station binary installed
2. **squad.yml syntax** — CLI provider example AND Antigravity example
3. **Initialization** — `squad-station init`, what it does
4. **Hook setup (canonical)** — `squad-station signal $TMUX_PANE` inline command for Claude Code Stop event and Gemini CLI AfterAgent event
5. **Notification hook setup** — `Notification` event with `permission_prompt` matcher (DOCS-03)
6. **Send a task** — `squad-station send <agent> --body "..."`
7. **Signal completion** — both manual and via hook
8. **Antigravity IDE mode** — when to use, what changes, squad.yml syntax

### Anti-Patterns to Avoid

- **Sending Enter after paste-buffer:** `paste-buffer` in tmux pastes content into the terminal as if typed — it does NOT automatically press Enter. The existing `send_keys_literal` sends Enter as a separate step. `inject_body` MUST also send Enter after paste. Use the existing `send_keys_literal` function just for the Enter key, or call `enter_args` directly.
- **Using `paste-buffer -p`:** The `-p` flag pastes to the current pane in the active window. Must use `-t <target>` to target a specific session/pane.
- **Not cleaning up temp files:** On error paths, temp file must still be removed. Use `let _ = std::fs::remove_file(...)` to ignore the cleanup result (cleanup failure is non-fatal).
- **Blocking load-buffer on large content:** `load-buffer` reads a file, not stdin — no size concern. `send-keys -l` with large text is what breaks (tmux argument length limit).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Unique temp file names | Custom timestamp hash | `uuid::Uuid::new_v4()` (already in Cargo.toml) | UUID v4 is collision-free, already available |
| Temp dir path | Hardcode `/tmp/` | `std::env::temp_dir()` | Portable across macOS and Linux |
| JSON parsing for PLAYBOOK examples | Custom parser | Show literal YAML/JSON in markdown code blocks | PLAYBOOK is documentation, not code |

**Key insight:** The injection mechanism itself is 3 tmux commands + file write. The only complexity is correct temp file cleanup on all code paths (success and error).

---

## Common Pitfalls

### Pitfall 1: paste-buffer Does Not Send Enter

**What goes wrong:** `tmux paste-buffer` pastes the content into the terminal buffer but does not simulate pressing Enter. If the agent's shell is waiting at a prompt, the task body appears but is not submitted.

**Why it happens:** `send-keys` literally sends keystrokes including Enter; `paste-buffer` only inserts text content.

**How to avoid:** After `inject_body` pastes the buffer, send Enter using the existing `enter_args`/`send_keys_literal` mechanism — or have `inject_body` call the enter step internally. The design in SOLUTION-DESIGN.md §6.5 does not mention Enter explicitly, but the current `send_keys_literal` sends Enter. The replacement must preserve this behavior.

**Warning signs:** Agent session shows task body but never processes it.

### Pitfall 2: Temp File Cleanup on Error Paths

**What goes wrong:** If `load-buffer` succeeds but `paste-buffer` fails, the temp file is orphaned in `/tmp/`.

**Why it happens:** Early `bail!()` returns skip cleanup code.

**How to avoid:** Always call `std::fs::remove_file` before every `bail!()` return, or use a RAII guard. Given Rust ownership, the simplest approach: capture path early, attempt cleanup in a defer-like pattern (or just remove before each bail).

**Warning signs:** `/tmp/` accumulating `squad-station-msg-*` files.

### Pitfall 3: paste-buffer Target Syntax

**What goes wrong:** `tmux paste-buffer -t session-name` may not work if the session has multiple windows/panes; tmux may not know which pane to target.

**Why it happens:** tmux target disambiguation — if `-t session` resolves to a session with multiple windows, it defaults to the current/active pane which may not be the agent's pane.

**How to avoid:** Use `-t <session>` targeting and rely on the fact that each agent session was created with a single window (via `new-session`). This is already the pattern for `send_keys_literal`. If needed, append `.0` suffix to target the first window/pane: `<session>:0.0`.

**Warning signs:** Content pasted to wrong pane in a multi-pane session.

### Pitfall 4: PLAYBOOK Hook JSON Shows Wrong Provider Key

**What goes wrong:** PLAYBOOK shows `"provider"` instead of `"tool"` in settings.json examples, or shows shell script path instead of inline command.

**Why it happens:** Project renamed `provider` → `tool` in the DB schema (GAP-03 decision #5) but the hook config in settings.json is independent of the DB schema — it just runs a shell command.

**How to avoid:** The hook command in settings.json is simply `"squad-station signal $TMUX_PANE"` — no reference to provider/tool field. The PLAYBOOK must show the final confirmed form from hooks/claude-code.sh which is already correct.

**Warning signs:** User confusion between squad.yml `tool:` field and settings.json hook command.

### Pitfall 5: PLAYBOOK squad.yml Uses Wrong Field Name

**What goes wrong:** PLAYBOOK example uses `provider:` instead of `tool:` in squad.yml.

**Why it happens:** docs/SOLUTION-DESIGN.md §2 still uses `provider:` in code blocks (the docs were written before the rename decision), but the actual `config.rs` and the live `squad.yml` use `tool:`.

**How to avoid:** Use the live `squad.yml` as the authoritative example, not the docs. The field in `config.rs` is `tool` — PLAYBOOK must match. Example from live `squad.yml`:
```yaml
orchestrator:
  tool: claude-code
  role: orchestrator
  model: claude-opus-4-5
```

---

## Code Examples

Verified patterns from existing codebase:

### Existing Arg-Builder Test Pattern (Reference for New Tests)
```rust
// Source: src/tmux.rs (existing)
#[test]
fn test_send_keys_args_have_literal_flag() {
    let args = send_keys_args("my-session", "hello world");
    assert_eq!(args[0], "send-keys");
    assert_eq!(args[3], "-l", "SAFE-02: -l flag must be present");
    assert_eq!(args[4], "hello world");
}
```

New tests must follow this exact structure:
```rust
#[test]
fn test_load_buffer_args() {
    let args = load_buffer_args("/tmp/squad-station-msg-abc");
    assert_eq!(args[0], "load-buffer");
    assert_eq!(args[1], "/tmp/squad-station-msg-abc");
    assert_eq!(args.len(), 2, "load-buffer takes only the path, no flags");
}

#[test]
fn test_paste_buffer_args() {
    let args = paste_buffer_args("my-agent");
    assert_eq!(args[0], "paste-buffer");
    assert_eq!(args[1], "-t");
    assert_eq!(args[2], "my-agent");
    assert_eq!(args.len(), 3);
}

#[test]
fn test_load_buffer_args_with_spaces_in_path() {
    // Path with spaces must be preserved as-is (passed as arg, not shell-expanded)
    let args = load_buffer_args("/tmp/my path/file");
    assert_eq!(args[1], "/tmp/my path/file");
}
```

### PLAYBOOK Hook JSON (Claude Code)
```json
// Source: docs/SOLUTION-DESIGN.md §6.1 + hooks/claude-code.sh (confirmed form)
{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "squad-station signal $TMUX_PANE"
      }
    ]
  }
}
```

### PLAYBOOK Hook JSON (Gemini CLI)
```json
// Source: docs/SOLUTION-DESIGN.md §6.2 (AfterAgent event)
{
  "hooks": {
    "AfterAgent": [
      {
        "type": "command",
        "command": "squad-station signal $TMUX_PANE"
      }
    ]
  }
}
```

### PLAYBOOK Notification Hook (Claude Code) — DOCS-03
```json
// Source: docs/VISION.md §2.3 (Notification event with permission_prompt matcher)
{
  "hooks": {
    "Notification": [
      {
        "matcher": "permission_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "hooks/claude-code-notify.sh"
          }
        ]
      }
    ]
  }
}
```

### PLAYBOOK squad.yml — Antigravity Provider (DOCS-02)
```yaml
# Source: squad.yml (live file) + docs/SOLUTION-DESIGN.md §2 (antigravity variant)
project: my-app

# Standard CLI orchestrator:
orchestrator:
  tool: claude-code
  role: orchestrator
  model: claude-opus-4-5
  description: >
    Main orchestrator. Delegates tasks, synthesizes results.

# Alternative: IDE-based orchestrator (Antigravity):
# orchestrator:
#   tool: antigravity
#   role: orchestrator
#   description: >
#     Orchestrator running inside Antigravity IDE.
#     Uses Manager View to poll and monitor tmux worker agents.

agents:
  - name: implement
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Implements features and fixes bugs"
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Shell scripts `hooks/claude-code.sh` | Inline `squad-station signal $TMUX_PANE` | Phase 10 (v1.3) | Simpler setup, no script path dependency |
| `send-keys -l` for all content | `load-buffer`/`paste-buffer` for body content | Phase 13 (v1.3) | Safe multiline delivery, no arg length limit |
| No PLAYBOOK or stale PLAYBOOK | Authoritative PLAYBOOK.md | Phase 13 (v1.3) | Single source of truth for users |

**Deprecated/outdated:**
- `hooks/claude-code.sh` and `hooks/gemini-cli.sh`: deprecated in file headers as of Phase 10; kept for reference only. PLAYBOOK must not reference them as the setup method.
- SOLUTION-DESIGN.md §2 still uses `provider:` in YAML examples — PLAYBOOK must use `tool:` (the actual field name in config.rs).

---

## Open Questions

1. **Does paste-buffer require Enter to be sent separately?**
   - What we know: `send_keys_literal` sends text then Enter as a separate `send-keys Enter` call. `paste-buffer` inserts content without a trailing newline/Enter.
   - What's unclear: Whether the AI tool (Claude Code, Gemini CLI) reads stdin or reads the terminal buffer. If it reads stdin, paste-buffer may not work at all.
   - Recommendation: Preserve the Enter step after paste-buffer. `inject_body` should call Enter via existing `enter_args` pattern after paste-buffer completes.

2. **PLAYBOOK location: `/PLAYBOOK.md` or `docs/PLAYBOOK.md`?**
   - What we know: `README.md` references `docs/PLAYBOOK.md`. GAP-09 says "Rewrite PLAYBOOK.md" without specifying path. No PLAYBOOK.md exists anywhere currently.
   - What's unclear: Whether planner should create at root or under `docs/`.
   - Recommendation: Create at `docs/PLAYBOOK.md` to match README.md's existing reference.

3. **Does `tmux paste-buffer` need `-p` flag or not?**
   - What we know: `-p` pastes to stdout (current pane). Without `-p`, it pastes to the target specified by `-t`.
   - Recommendation: Use `-t <target>` without `-p`. This matches how `send-keys -t` targets sessions.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in + tokio-test (Cargo.toml dev-deps) |
| Config file | None (standard `cargo test`) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && ./tests/e2e_cli.sh` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TMUX-01 | `load_buffer_args` returns correct args | unit | `cargo test test_load_buffer_args` | ❌ Wave 0 |
| TMUX-01 | `paste_buffer_args` returns correct args | unit | `cargo test test_paste_buffer_args` | ❌ Wave 0 |
| TMUX-01 | `inject_body` arg construction (no tmux invocation) | unit | `cargo test test_inject_body_args` | ❌ Wave 0 |
| TMUX-01 | temp file is cleaned up (mock path) | unit | `cargo test test_inject_body_cleanup` | ❌ Wave 0 |
| TMUX-02 | `send.rs` calls `inject_body` not `send_keys_literal` for body | integration | `cargo test test_send_uses_inject_body` | ❌ Wave 0 |
| DOCS-01 | PLAYBOOK.md exists with signal $TMUX_PANE | manual | `test -f docs/PLAYBOOK.md` | ❌ Wave 0 |
| DOCS-02 | PLAYBOOK.md contains antigravity provider section | manual | `grep -q "antigravity" docs/PLAYBOOK.md` | ❌ Wave 0 |
| DOCS-03 | PLAYBOOK.md covers Notification hook | manual | `grep -q "Notification" docs/PLAYBOOK.md` | ❌ Wave 0 |

Note: TMUX-01/TMUX-02 unit tests test arg-builder functions only (no live tmux needed). Live tmux injection is covered by the existing `e2e_cli.sh` test which requires a running tmux.

### Sampling Rate

- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test`
- **Phase gate:** `cargo test` green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src/tmux.rs` — add `load_buffer_args`, `paste_buffer_args`, `inject_body` + their unit tests (all new code in existing file)
- [ ] `src/commands/send.rs` — replace `send_keys_literal` call with `inject_body`
- [ ] `docs/PLAYBOOK.md` — create file (does not exist yet)

*(No new test files needed — new unit tests go in the existing `#[cfg(test)]` block in `src/tmux.rs`)*

---

## Sources

### Primary (HIGH confidence)

- Codebase: `src/tmux.rs` — existing arg-builder pattern, `send_keys_literal` implementation
- Codebase: `src/commands/send.rs` — current injection call site (line 58)
- Codebase: `hooks/claude-code.sh` — canonical inline hook form (confirmed form: `squad-station signal $TMUX_PANE`)
- Codebase: `docs/SOLUTION-DESIGN.md §6.5` — safe injection design spec (load-buffer/paste-buffer flow)
- Codebase: `docs/SOLUTION-DESIGN.md §6.1-6.4` — hook system design, settings.json format
- Codebase: `squad.yml` (live) — authoritative `tool:` field name
- Codebase: `Cargo.toml` — confirms `uuid 1.8` already available; `tempfile 3` is dev-dep only

### Secondary (MEDIUM confidence)

- `docs/GAP-ANALYSIS.md GAP-17` — confirms temp file approach: "write body to temp file → tmux load-buffer → tmux paste-buffer → cleanup"
- `docs/TECH-STACK.md §5` — safety checklist item #9 confirms `load-buffer`/`paste-buffer` as solution

### Tertiary (LOW confidence)

- tmux man page knowledge (from training data): `load-buffer [path]` loads file content into paste buffer; `paste-buffer -t target` pastes buffer to target pane. Needs runtime verification via e2e test.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already in Cargo.toml; implementation pattern is an extension of existing tmux.rs code
- Architecture: HIGH — design fully specified in docs/SOLUTION-DESIGN.md §6.5; arg-builder pattern is established and consistent
- Pitfalls: MEDIUM — Enter-after-paste-buffer pitfall identified from code analysis; tmux target syntax pitfall from general tmux knowledge (LOW sub-component)
- Documentation content: HIGH — all content sourced from existing docs + live config files

**Research date:** 2026-03-09
**Valid until:** 2026-06-09 (stable domain — tmux API and project design are unlikely to change)
