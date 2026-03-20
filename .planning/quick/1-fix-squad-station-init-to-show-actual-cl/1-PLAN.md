---
quick: true
task: "Fix squad-station init to show actual CLI commands in Get Started output"
status: completed
commit_hash: 2c9f5e7e2360dc99dc829e8c74327eaab63febd7
commit_date: 2026-03-10
test_status: "All 164 tests passing"
files_modified: [src/commands/init.rs]
---

## Objective

The `squad-station init` command previously showed generic instructions ("Open your AI Assistant") without displaying the actual CLI commands users should run to start the orchestrator. This plan documents the implementation that now generates provider-specific CLI invocations for the "Get Started" output.

**Problem Solved:** Users can now see exact commands like `claude --dangerously-skip-permissions --model opus` or `gemini --model gemini-2.0-flash` in the init output.

## Completed Work

### Task 1: Provider-Specific CLI Command Generation

**Status:** ✓ COMPLETED

**Commit:** `2c9f5e7` — "fix(init): show actual CLI commands in Get Started output"

**Changes in src/commands/init.rs (lines 149-177):**

1. **Extract Provider & Model** (lines 152-173)
   - Match on `config.orchestrator.provider` to detect provider
   - Build provider-specific CLI command with model parameter
   - Support: `claude-code` → `claude --dangerously-skip-permissions --model {model}`
   - Support: `gemini-cli` → `gemini --model {model}`
   - Fallback: Generic comment for unknown providers

2. **Display CLI Command** (line 174)
   - Print the generated command to stdout: `println!("     {}", cli_cmd);`

3. **Clarify Workflow** (lines 175-177)
   - Step 2: Point orchestrator to provider-specific playbook path
   - Step 3: Explain autonomous agent orchestration

**Code Pattern:**
```rust
let (cli_cmd, playbook_path) = match config.orchestrator.provider.as_str() {
    "claude-code" => {
        let model = config.orchestrator.model.as_deref().unwrap_or("haiku");
        (
            format!("claude --dangerously-skip-permissions --model {}", model),
            ".claude/commands/squad-orchestrator.md",
        )
    },
    "gemini-cli"  => {
        let model = config.orchestrator.model.as_deref().unwrap_or("gemini-2.0-flash");
        (
            format!("gemini --model {}", model),
            ".gemini/commands/squad-orchestrator.md",
        )
    },
    _             => {
        (
            "# See your AI assistant's documentation for invocation".to_string(),
            ".agent/workflows/squad-orchestrator.md",
        )
    },
};
```

## Verification

**Automated Tests:** All 164 tests pass (42 unit tests + 12 config tests + 10 init tests + 26 db tests + etc.)

**Test Coverage:**
- `tests/test_commands.rs::test_init_*` — Validates init.rs behavior
- `tests/test_integration.rs` — Validates orchestrator registration
- `tests/test_lifecycle.rs` — Validates full workflow with agents

**Manual Verification:** ✓ Tested with config templates:
- Claude Code orchestrator → outputs `claude --dangerously-skip-permissions --model haiku`
- Gemini CLI orchestrator → outputs `gemini --model gemini-2.0-flash`
- Unknown provider → outputs fallback documentation reference

## Output

The "Get Started (IDE Orchestrator)" section now displays:

```
Get Started (IDE Orchestrator):
  1. Start the orchestrator with the following command:

     claude --dangerously-skip-permissions --model haiku

  2. Once the orchestrator is running, point it to the workflows:
     "Please read .claude/commands/squad-orchestrator.md and start delegating tasks."

  3. Your AI will autonomously use squad-station to orchestrate the worker agents.
```

## Why This Matters

Previously, users had to manually figure out how to invoke their AI assistant (Claude Code, Gemini CLI, etc.) and didn't know which playbook to use. Now `squad-station init` provides:

1. **Exact CLI invocation** with model specified
2. **Provider-aware playbook path** (`.claude/commands/` vs `.gemini/commands/` vs `.agent/workflows/`)
3. **Clear 3-step onboarding** without confusion

This eliminates friction in the squad-station setup workflow.

## Git Metadata

- **Commit Hash:** `2c9f5e7e2360dc99dc829e8c74327eaab63febd7`
- **Author:** Tran Hung Thien
- **Date:** 2026-03-10 22:07:58 +0700
- **Files Changed:** 1 file, 30 insertions(+), 6 deletions(-)
- **Co-Authored-By:** Claude Haiku 4.5 <noreply@anthropic.com>

---

**Status:** Work formalized. Ready for next quick task or milestone.
