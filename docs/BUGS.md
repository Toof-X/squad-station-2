# Squad Station — Bug Report

**Tested:** 2026-03-15
**Binary:** squad-station v0.2.0 (target/release/squad-station)
**Test project:** squad-station-landing-page (tmux session: squad-station-testing)

---

## BUG-01: Signal completes wrong task (LIFO vs FIFO mismatch) [CRITICAL]

**Description:** `signal` completes the **newest** processing task (`ORDER BY created_at DESC`), but `peek` returns the **oldest** processing task (`ORDER BY created_at ASC` with priority). When an agent has multiple tasks, it peeks task A, works on it, signals, but task B (newer) gets marked completed instead.

**Reproduce:**
```bash
squad-station send squad-station-implement --body "task A"
squad-station send squad-station-implement --body "task B"
squad-station peek squad-station-implement   # Returns "task A" (oldest)
squad-station signal squad-station-implement  # Completes "task B" (newest!)
```

**Location:** `src/db/messages.rs:65` — `ORDER BY created_at DESC LIMIT 1` should be `ASC`.

---

## BUG-02: Signal sets agent to idle even with remaining tasks [HIGH]

**Description:** After `signal` completes one task, the agent status is unconditionally set to `idle`, even when other `processing` tasks remain in the queue. The `status` command then shows the agent as "idle" with "N pending" — contradictory.

**Reproduce:**
```bash
squad-station send squad-station-implement --body "task 1"
squad-station send squad-station-implement --body "task 2"
squad-station signal squad-station-implement
squad-station status  # Shows: implement idle | 1 pending
```

**Expected:** Agent should remain `busy` if processing tasks remain.

**Location:** `src/commands/signal.rs:102-108` — needs a check for remaining processing messages before setting idle.

---

## BUG-03: Agent names require full project prefix — no short name resolution [MEDIUM]

**Description:** Agents defined as `implement` in `squad.yml` are stored as `squad-station-implement` in DB. All commands require the full prefixed name. Short names fail with "Agent not found".

**Reproduce:**
```bash
squad-station send implement --body "test"       # Error: Agent not found: implement
squad-station list --agent implement              # No messages found (silent miss)
squad-station peek implement                      # No pending tasks (silent miss)
```

**Expected:** Accept both `implement` and `squad-station-implement`. At minimum, suggest the full name when short name fails.

---

## BUG-04: `signal` for nonexistent agent silently succeeds [MEDIUM]

**Description:** `squad-station signal nonexistent-agent` produces no output and exits 0. By design for hook context (HOOK-03), but confusing for manual CLI usage.

**Reproduce:**
```bash
squad-station signal nonexistent  # No output, exit 0
```

**Note:** This is intentional for hooks but should at least print a message when running interactively (TTY detected).

**Location:** `src/commands/signal.rs:42-44` — silent `return Ok(())` when agent not found.

---

## BUG-05: `peek` doesn't validate agent exists [LOW]

**Description:** `peek nonexistent-agent` says "No pending tasks for nonexistent" instead of "Agent not found". Misleading — user thinks agent exists but has no tasks.

**Reproduce:**
```bash
squad-station peek nonexistent  # "No pending tasks for nonexistent"
```

**Expected:** "Agent not found: nonexistent"

---

## BUG-06: `from_agent` hardcoded to "orchestrator" (no prefix) [LOW]

**Description:** `send` command hardcodes `from_agent` as `"orchestrator"` (line 40 of send.rs), while the orchestrator agent is registered as `squad-station-orchestrator`. The FROM column in `list` shows "orchestrator" while TO shows the full prefixed name.

**Location:** `src/commands/send.rs:40` — hardcoded `"orchestrator"` string.

---

## BUG-07: Can send tasks to orchestrator-role agents [LOW]

**Description:** No guard preventing `squad-station send squad-station-orchestrator --body "test"`. The orchestrator is a coordinator, not a task receiver.

**Reproduce:**
```bash
squad-station send squad-station-orchestrator --body "some task"  # Succeeds
```

**Expected:** "Cannot send tasks to orchestrator-role agents" or similar guard.

---

## BUG-08: Can send empty body tasks [LOW]

**Description:** `squad-station send agent --body ""` succeeds and creates a task with empty content. No validation on body content.

**Reproduce:**
```bash
squad-station send squad-station-implement --body ""  # Succeeds with empty task
```

---

## BUG-09: `list --status` help text says "pending" but status doesn't exist [LOW]

**Description:** `list --help` says `--status` accepts `(pending, completed)`, but messages go directly to `processing` status (never `pending`). `--status processing` works but isn't documented.

**Reproduce:**
```bash
squad-station list --status pending     # Always "No messages found"
squad-station list --status processing  # Works (undocumented)
```

**Location:** `src/cli.rs` — help text for `--status` option.

---

## BUG-10: `status` shows "pending" count but actual status is "processing" [LOW]

**Description:** The `status` command displays "N pending" for each agent, but the underlying messages have status `processing`, not `pending`. Terminology mismatch between display and data model.

---

## BUG-11: `init` reports "0 agent(s)" when sessions already exist [LOW]

**Description:** Running `init` when tmux sessions already exist shows "Initialized squad with 0 agent(s)" even though 3 agents are registered. The count only reflects newly launched sessions.

**Reproduce:**
```bash
squad-station init    # First time: "3 agent(s)"
squad-station init    # Second time: "0 agent(s)" — misleading
```

---

## BUG-12: Missing squad.yml error is not user-friendly [LOW]

**Description:** Running any command without `squad.yml` in CWD shows raw OS error: "No such file or directory (os error 2)". Should say "squad.yml not found in current directory".

---

## BUG-13: `status_updated_at` timestamp format inconsistency in JSON [LOW]

**Description:** In `status --json` output, `status_updated_at` uses different formats: RFC3339 with microseconds (`2026-03-15T11:10:37.402493+00:00`) for agents updated via commands, but `2026-03-15 11:10:08` (no timezone, no T separator) for agents set during `init`.

---

## BUG-14: `notify` messages not persisted to database [NOTE]

**Description:** `notify` sends a tmux message to the orchestrator but doesn't create a record in the messages table. Notifications are ephemeral and untracked. May be by design, but worth noting for audit trail purposes.

---

## BUG-15: `register` doesn't prefix agent name with project [NOTE]

**Description:** `init` registers agents with `{project}-{name}` prefix, but `register` uses the raw name. Inconsistent naming convention.

**Reproduce:**
```bash
squad-station register test-agent --role worker  # Stored as "test-agent"
squad-station agents  # Shows "test-agent" alongside "squad-station-implement"
```

---

## BUG-16: Remove `tmux-session` from squad.yml — session names must follow `{project}-{name}` convention [UPGRADE]

**Description:** The `tmux-session` field in `squad.yml` agent configs is silently ignored by serde (not present in the `AgentConfig` struct). Session names are always derived as `{project}-{name}` in `init.rs:59`. Users may think they are customizing session names via this field, but it has no effect.

**Required changes:**

1. **Remove `tmux-session` from squad.yml** — it is dead config. The test project has:
   ```yaml
   agents:
     - name: implement
       tmux-session: squad-implement  # <-- silently ignored, does nothing
   ```

2. **Enforce `{project}-{name}` as the sole session naming convention** — this is already the actual behavior, but it should be explicit and documented.

3. **Sanitize special characters in derived session names.** The project name or agent name may contain characters that break tmux session targeting:
   - `.` (dot) — tmux interprets as `session.window` separator. A session created with `.` in the name cannot be addressed by `has-session -t`, `send-keys -t`, or any `-t` targeting. The session becomes unreachable.
   - `:` (colon) — tmux silently converts to `_`. Session is created as `test_session` instead of `test:session`, causing name mismatch between DB and tmux.
   - `"` (double quote) — works in tmux but can break shell expansion in hook commands like `squad-station signal $(tmux display-message -p '#S')` if the session name contains unescaped quotes.

   **Fix:** Add a `sanitize_session_name()` function that replaces `.`, `:`, and `"` with `-` (or rejects them with a clear error) before passing to tmux. Apply this in `init.rs` when deriving `agent_name` and in `tmux.rs:launch_agent()`.

**Reproduce (dot problem):**
```bash
# If project name were "my.app" and agent "worker":
tmux new-session -d -s 'my.app-worker' zsh    # Session created OK
tmux has-session -t 'my.app-worker'            # FAILS: "can't find window: my"
# Session is orphaned — cannot be targeted, killed, or used
```

**Location:**
- `src/config.rs:53-61` — `AgentConfig` struct (no `tmux_session` field, serde silently drops it)
- `src/commands/init.rs:59` — `format!("{}-{}", config.project, role_suffix)` — needs sanitization
- `src/tmux.rs` — all `-t` targeting functions assume session name is tmux-safe
