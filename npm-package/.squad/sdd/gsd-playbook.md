# GSD (Get Shit Done) — Operational Playbook v1.26

> This is an **action-oriented** document. Practical operational handbook for the GSD v1.26 system for developers — from installation, Day 1 workflow, to best practices and advanced troubleshooting.

---

## Part I: Installation & Environment Setup

### 1.1. Quick Installation (30 seconds)

```bash
npx get-shit-done-cc@latest
```

The installer will ask you 2 questions:
1. **Runtime:** Claude Code, OpenCode, Gemini, Codex, Copilot, Antigravity, or All
2. **Location:** Global (all projects) or Local (current project only)

**Verify successful installation:**

| Runtime | Verification Command |
|---|---|
| Claude Code | `/gsd:help` |
| Gemini CLI | `/gsd:help` |
| OpenCode | `/gsd-help` |
| Codex | `$gsd-help` |
| Copilot CLI | `/gsd:help` |
| Antigravity | `/gsd:help` |

### 1.2. Non-Interactive Installation

```bash
# Claude Code
npx get-shit-done-cc --claude --global   # → ~/.claude/
npx get-shit-done-cc --claude --local    # → ./.claude/

# Gemini CLI
npx get-shit-done-cc --gemini --global   # → ~/.gemini/

# OpenCode
npx get-shit-done-cc --opencode --global # → ~/.config/opencode/

# Codex
npx get-shit-done-cc --codex --global    # → ~/.codex/

# Copilot CLI (NEW v1.23)
npx get-shit-done-cc --copilot --global

# Antigravity (NEW v1.25)
npx get-shit-done-cc --antigravity --global

# All runtimes
npx get-shit-done-cc --all --global
```

### 1.3. Updating

```bash
npx get-shit-done-cc@latest
```

From v1.17+, the installer automatically backs up local modifications to `gsd-local-patches/`. Use `/gsd:reapply-patches` to restore them.

> **TIP:** `/gsd:update` is the way to update from within a session — it shows a changelog preview and targets the correct runtime directory.

### 1.4. Skip Permissions Mode (Recommended)

```bash
claude --dangerously-skip-permissions
```

> **TIP:** GSD is designed for continuous automation. Stopping to approve `date` and `git commit` 50 times per session breaks the "flow" and significantly reduces productivity.

**Alternative — Granular Permissions:**

```json
{
  "permissions": {
    "allow": [
      "Bash(date:*)", "Bash(echo:*)", "Bash(cat:*)", "Bash(ls:*)",
      "Bash(mkdir:*)", "Bash(wc:*)", "Bash(head:*)", "Bash(tail:*)",
      "Bash(sort:*)", "Bash(grep:*)", "Bash(tr:*)",
      "Bash(git add:*)", "Bash(git commit:*)", "Bash(git status:*)",
      "Bash(git log:*)", "Bash(git diff:*)", "Bash(git tag:*)"
    ]
  }
}
```

### 1.5. Security — Configuring Deny List

> ⚠️ **Mandatory** configuration before using GSD with any project containing secrets.

Add to `.claude/settings.json`:

```json
{
  "permissions": {
    "deny": [
      "Read(.env)", "Read(.env.*)",
      "Read(**/secrets/*)", "Read(**/*credential*)",
      "Read(**/*.pem)", "Read(**/*.key)"
    ]
  }
}
```

### 1.6. Docker/Container/WSL Environments

```bash
# Docker — tilde path overflow
CLAUDE_CONFIG_DIR=/home/youruser/.claude npx get-shit-done-cc --global

# WSL — GSD will detect WSL + Windows Node.js mismatch and warn
```

---

## Part II: Day 1 — New Project (Greenfield)

### 2.1. Complete A→Z Workflow

```bash
# Step 0: Launch Claude Code
claude --dangerously-skip-permissions

# Step 1: Initialize project (Q&A → Research → Requirements → Roadmap)
/gsd:new-project

# Step 2: Clear context, begin phase loop
/clear

# Steps 3-7: Loop for each phase
/gsd:discuss-phase 1        # Lock in preferences
/gsd:ui-phase 1             # UI design contract (frontend phases)
/gsd:plan-phase 1           # Research + Plan + Verify
/gsd:execute-phase 1        # Parallel wave execution + regression gate
/gsd:verify-work 1          # Manual UAT
/gsd:ui-review 1            # Visual audit (frontend phases)
/clear

# Repeat for phase 2, 3, ...

# When all phases are complete
/gsd:audit-milestone         # Check DoD
/gsd:ship                    # Create PR from planning artifacts
/gsd:complete-milestone      # Archive + Tag release
```

### 2.2. Quick Flow — `/gsd:next` (NEW v1.26)

Can't remember the next step? Just run:

```bash
/gsd:next                    # Auto-advance to next logical step
```

### 2.3. Initialize from Existing Document

```bash
/gsd:new-project --auto @prd.md
```

The system automatically runs research → requirements → roadmap from the document. Continue the normal workflow from discuss-phase.

### 2.4. Autonomous Mode

Want to run all remaining phases automatically:

```bash
/gsd:autonomous               # All remaining phases
/gsd:autonomous --from 3       # Start from phase 3
```

---

## Part III: Day 1 — Existing Project (Brownfield)

### 3.1. Map Codebase First

```bash
/gsd:map-codebase                   # Full codebase analysis (4 parallel mappers)
/gsd:map-codebase auth              # Focus on specific area
```

### 3.2. Output

```
codebase/STACK.md          → Tech stack, dependencies
codebase/ARCHITECTURE.md   → Code structure, patterns
codebase/CONVENTIONS.md    → Naming, style, practices
codebase/CONCERNS.md       → Tech debt, security, risks
```

---

## Part IV: Daily Operations — Phase Loop

### 4.1. Discuss Phase — "Shape the Build"

```bash
/gsd:discuss-phase 1
/gsd:discuss-phase 3 --auto    # Auto-select defaults
/gsd:discuss-phase --batch     # Grouped question intake (v1.23)
```

**Code-aware discuss (v1.22):** GSD analyzes relevant source files before asking — doesn't re-ask things already clear in the code.

### 4.2. UI Design Phase (NEW v1.23)

```bash
/gsd:ui-phase 2                # Generate UI design contract
```

- Detects design system state (shadcn, Tailwind config, existing tokens)
- Auto-offers shadcn initialization for React/Next.js/Vite
- Registry safety gate for third-party components
- Validation loop (max 2 iterations): BLOCK/FLAG/PASS

### 4.3. Plan Phase — "Blueprint the Build"

```bash
/gsd:plan-phase 1
/gsd:plan-phase 3 --skip-research    # Skip research
/gsd:plan-phase --skip-verify        # Skip plan checker
/gsd:plan-phase --auto               # Non-interactive
```

**v1.25:** Plan-phase asks user about research instead of silently deciding.

**Before planning — see what AI intends:**

```bash
/gsd:list-phase-assumptions 1
```

### 4.4. Execute Phase — "Build It"

```bash
/gsd:execute-phase 1
```

**v1.23+:** Node repair operator auto RETRY/DECOMPOSE/PRUNE when task verification fails.

**v1.26:** Cross-phase regression gate — runs prior phases' test suites after execution.

**v1.26:** Interactive executor mode for pair-programming style execution.

### 4.5. Verify Work — "Test It"

```bash
/gsd:verify-work 1
```

- Cold-start smoke test auto-inject for phases modifying server/database/seed/startup files (v1.22.3)
- Debug agents auto-diagnose root cause + create fix plans
- Debug sessions save to persistent knowledge base (v1.24)

### 4.6. UI Review (NEW v1.23)

```bash
/gsd:ui-review 1               # 6-pillar visual audit
```

6 Pillars (scored 1-4): Copywriting, Visuals, Color, Typography, Spacing, Experience Design.

### 4.7. Ship (NEW v1.26)

```bash
/gsd:ship 4                    # Ship phase 4 as PR
/gsd:ship 4 --draft            # Draft PR
```

---

## Part V: Milestone Management

### 5.1. Audit → Ship → Complete

```bash
/gsd:audit-milestone           # Check DoD
/gsd:plan-milestone-gaps       # Create phases for gaps
/gsd:ship                      # Create PR
/gsd:complete-milestone        # Archive + Tag release
/gsd:new-milestone [name]      # Start new milestone
```

### 5.2. Stats Dashboard (NEW v1.23)

```bash
/gsd:stats                     # Phases, plans, requirements, git metrics, timeline
```

---

## Part VI: Quick Mode — Ad-hoc Tasks

### 6.1. Quick Task (Enhanced v1.24)

```bash
/gsd:quick                           # Basic quick task
/gsd:quick --discuss                 # Pre-planning discussion
/gsd:quick --research                # Spawn focused researcher (v1.24)
/gsd:quick --full                    # Plan checking + verification
/gsd:quick --discuss --research --full  # Maximum quality
```

Flags are composable. Quick mode uses `YYMMDD-xxx` timestamp IDs (v1.23).

---

## Part VII: Productivity Commands (NEW)

### 7.1. Natural Language Router (v1.25)

```bash
/gsd:do                              # Describe what you want → auto-route
```

### 7.2. Zero-Friction Notes (v1.25)

```bash
/gsd:note "Consider caching strategy"  # Capture idea
/gsd:note list                         # List all notes
/gsd:note promote 3                    # Promote to structured todo
/gsd:note --global "..."               # Global scope
```

### 7.3. Developer Profiling (v1.26)

```bash
/gsd:profile-user                      # Analyze sessions → behavioral profile
/gsd:profile-user --questionnaire      # Interactive fallback
/gsd:profile-user --refresh            # Re-analyze
```

**Output:** `USER-PROFILE.md`, `/gsd:dev-preferences`, `CLAUDE.md` profile section.

### 7.4. Session Management

```bash
/gsd:pause-work               # HANDOFF.json + continue-here.md
/gsd:resume-work               # Full context restoration
/gsd:progress                  # "Where am I? What's next?"
/gsd:next                      # Auto-advance to next step (v1.26)
/gsd:session-report            # Session summary (v1.26)
```

---

## Part VIII: Cheat Sheet — Quick Reference (42 commands)

### Group 1: Initialization

| Command | Description |
|---|---|
| `/gsd:map-codebase [area]` | Analyze existing codebase (4 parallel mappers) |
| `/gsd:new-project [--auto @file.md]` | Initialize project: Q&A → Research → Roadmap |
| `/gsd:new-milestone [name]` | Start new milestone for current project |

### Group 2: Core Phase Loop

| Command | Description |
|---|---|
| `/gsd:discuss-phase [N] [--auto] [--batch]` | Lock in preferences before planning |
| `/gsd:ui-phase [N]` | UI design contract (frontend phases) |
| `/gsd:plan-phase [N] [--auto] [--skip-research] [--skip-verify]` | Research + Plan + Verify |
| `/gsd:execute-phase <N>` | Parallel wave execution + node repair + regression gate |
| `/gsd:verify-work [N]` | Manual UAT + auto-diagnosis |
| `/gsd:ui-review [N]` | 6-pillar visual audit (frontend) |
| `/gsd:validate-phase [N]` | Retroactive test coverage audit (Nyquist) |
| `/gsd:ship [N] [--draft]` | Create PR from planning artifacts |

### Group 3: Milestone Management

| Command | Description |
|---|---|
| `/gsd:audit-milestone` | Check DoD |
| `/gsd:complete-milestone` | Archive + Tag release |
| `/gsd:plan-milestone-gaps` | Create phases for gaps from audit |
| `/gsd:stats` | Project statistics dashboard |

### Group 4: Phase Management

| Command | Description |
|---|---|
| `/gsd:add-phase` | Add phase to end of roadmap |
| `/gsd:insert-phase [N]` | Insert emergency phase (decimal numbering) |
| `/gsd:remove-phase [N]` | Remove phase + renumber |
| `/gsd:list-phase-assumptions [N]` | View Claude's intended approach |
| `/gsd:research-phase [N]` | Dedicated deep research |
| `/gsd:add-tests [N]` | Generate tests for completed phase |

### Group 5: Session & Navigation

| Command | Description |
|---|---|
| `/gsd:progress` | Where am I? What's next? |
| `/gsd:next` | Auto-advance to next logical step |
| `/gsd:resume-work` | Restore context from previous session |
| `/gsd:pause-work` | HANDOFF.json + continue-here.md |
| `/gsd:session-report` | Session summary |
| `/gsd:help` | All commands |
| `/gsd:update` | Update GSD + changelog preview |

### Group 6: Utilities

| Command | Description |
|---|---|
| `/gsd:quick [--discuss] [--research] [--full]` | Ad-hoc task with GSD guarantees |
| `/gsd:autonomous [--from N]` | Run all remaining phases autonomously |
| `/gsd:do` | Freeform text → auto-route to right command |
| `/gsd:note [text/list/promote N] [--global]` | Zero-friction idea capture |
| `/gsd:debug [desc]` | Systematic debugging + persistent knowledge base |
| `/gsd:profile-user [--questionnaire] [--refresh]` | Developer behavioral profile |
| `/gsd:add-todo [desc]` | Jot down an idea |
| `/gsd:check-todos` | View pending todos |
| `/gsd:settings` | Configure workflow + model |
| `/gsd:set-profile <profile>` | Switch quality/balanced/budget/inherit |
| `/gsd:health [--repair]` | Check + repair `.planning/` integrity |
| `/gsd:cleanup` | Archive completed milestone directories |
| `/gsd:reapply-patches` | Restore local edits after update |
| `/gsd:join-discord` | Join community |

---

## Part IX: Strategic Configuration

### 9.1. Model Profiles

```bash
/gsd:set-profile quality       # Max quality, Opus heavy
/gsd:set-profile balanced      # Default - Opus for planning only
/gsd:set-profile budget        # Cost-effective - Sonnet/Haiku
/gsd:set-profile inherit       # Use runtime's current model (OpenCode)
```

**v1.24:** `/gsd:set-profile` runs as a programmatic script — completes in seconds instead of 30-40s LLM-driven.

### 9.2. Workflow Presets

| Scenario | mode | granularity | profile | research | plan_check | verifier | ui_phase | node_repair |
|---|---|---|---|---|---|---|---|---|
| Prototyping | yolo | coarse | budget | off | off | off | off | off |
| Normal dev | interactive | standard | balanced | on | on | on | on | on |
| Production | interactive | fine | quality | on | on | on | on | on |

### 9.3. Context Window Control (v1.25)

Disable context monitor warnings:

```json
// .planning/config.json
{ "hooks": { "context_warnings": false } }
```

---

## Part X: Troubleshooting

### 10.1. Command Not Found

- **Restart runtime** to reload commands/skills
- **Verify files:**
  - Claude: `~/.claude/commands/gsd/` (global) or `./.claude/commands/gsd/` (local)
  - Codex: `~/.codex/skills/gsd-*/SKILL.md`
  - OpenCode: `.config/opencode/` config
  - Copilot: Maps to GitHub Copilot tools
  - Antigravity: Agent Skills format
- **Reinstall:** `npx get-shit-done-cc@latest`

### 10.2. "Project already initialized"

`.planning/PROJECT.md` already exists. If you want to start over: delete the `.planning/` directory.

### 10.3. AI Quality Degradation

```bash
/clear                         # Clear context window
/gsd:resume-work               # Restore state from files
```

### 10.4. Plans Going Wrong Direction

```bash
/gsd:list-phase-assumptions 1  # See what Claude intends
/gsd:discuss-phase 1           # Correct direction with preferences
/gsd:plan-phase 1              # Re-plan
```

### 10.5. Execution Creates Stubs/Incomplete Code

- Plans too large → break them down (2-3 tasks/plan max)
- Check `read_first` section has enough context (v1.23+)

### 10.6. Node Repair Exhausted Budget

- Increase `workflow.node_repair_budget` in config.json
- Or fix manually → re-execute

### 10.7. Agent Suggests Non-Existent Commands

Fixed v1.26: Agent no longer suggests `/gsd:transition` (doesn't exist). If still occurring, run `/gsd:update`.

### 10.8. Costs Too High

```bash
/gsd:set-profile budget        # Switch to Sonnet/Haiku
/gsd:settings                  # Turn off research, plan_check, verifier
```

### 10.9. Windows-specific Issues

- `@file:` protocol resolution for large payloads (>50KB) — fixed v1.22.4
- EPERM/EACCES when scanning protected directories — fixed v1.24
- WSL + Windows Node.js mismatch → GSD will detect and warn (v1.23)
- CRLF frontmatter parsing — fixed v1.26

### 10.10. Uninstall

```bash
npx get-shit-done-cc --claude --global --uninstall
npx get-shit-done-cc --opencode --global --uninstall
npx get-shit-done-cc --codex --global --uninstall
npx get-shit-done-cc --copilot --global --uninstall
npx get-shit-done-cc --antigravity --global --uninstall
```

---

## Part XI: Best Practices from the Field

### 11.1. Gold Rules — 9 Golden Rules

1. **`/clear` between phases.** Clean context window = peak quality.
2. **Invest time in Discuss.** The clearer you are, the more accurate AI becomes.
3. **Vertical Slices > Horizontal Layers.** Split features E2E, don't split by layer.
4. **Use `/gsd:progress` or `/gsd:next` frequently.** These are your compass.
5. **No manual debugging.** Use `/gsd:verify-work` or `/gsd:debug` to let AI self-diagnose.
6. **Commit initial docs.** `commit_docs: true` so team members and future-you understand context.
7. **Configure deny list BEFORE starting.** Defense-in-depth for secrets.
8. **Use `/gsd:ui-phase` for frontend.** UI design contract prevents "vibe-design".
9. **Use `/gsd:note` when you have ideas.** Capture ideas immediately, promote later.

### 11.2. Anti-Patterns — What to Avoid

| ❌ Anti-Pattern | ✅ Best Practice |
|---|---|
| Accepting defaults through Discuss too quickly | Answer in detail, describe your personal vision |
| Not using `/clear` between phases | Clear context after every verify |
| Manual debugging when verify fails | Let GSD spawn debug agents |
| Plans too large (5+ tasks) | Keep 2-3 tasks/plan, use `fine` granularity |
| Skipping audit before complete | ALWAYS audit → plan gaps → complete |
| Skip `/gsd:ui-phase` for frontend | UI-SPEC creates design contract, reduces revisions |
| Not using `/gsd:note` | Ideas are lost when context is cleared |

---

## Resources

| Resource | Link |
|---|---|
| **User Guide** | [User Guide](https://github.com/gsd-build/get-shit-done/blob/main/docs/USER-GUIDE.md) |
| **GitHub** | [gsd-build/get-shit-done](https://github.com/gsd-build/get-shit-done) |
| **Discord** | [Join Community](https://discord.gg/gsd) |