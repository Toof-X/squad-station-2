#!/bin/bash
# hooks/test-notify-hooks.sh -- TDD test script for notification hooks
# RED: Run before creating the hook files to verify failures
# GREEN: Run after creating to verify all pass

set -euo pipefail

HOOKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PASS=0
FAIL=0

run_test() {
    local name="$1"
    local result="$2"
    if [ "$result" = "ok" ]; then
        echo "  PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $name -- $result"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== Testing claude-code-notify.sh ==="

# Test: file exists
if [ -f "$HOOKS_DIR/claude-code-notify.sh" ]; then
    run_test "claude-code-notify.sh exists" "ok"
else
    run_test "claude-code-notify.sh exists" "file not found"
fi

# Test: file is executable
if [ -x "$HOOKS_DIR/claude-code-notify.sh" ]; then
    run_test "claude-code-notify.sh is executable" "ok"
else
    run_test "claude-code-notify.sh is executable" "not executable"
fi

# Test: syntax valid
if bash -n "$HOOKS_DIR/claude-code-notify.sh" 2>/dev/null; then
    run_test "claude-code-notify.sh bash -n passes" "ok"
else
    run_test "claude-code-notify.sh bash -n passes" "syntax error"
fi

# Test: exits 0 when TMUX_PANE not set (simulating no-tmux environment)
if env -i PATH="$PATH" bash "$HOOKS_DIR/claude-code-notify.sh" <<< '{}' 2>/dev/null; then
    run_test "claude-code-notify.sh exits 0 without TMUX_PANE" "ok"
else
    run_test "claude-code-notify.sh exits 0 without TMUX_PANE" "non-zero exit"
fi

# Test: contains 'exit 0' (every path)
if grep -q 'exit 0' "$HOOKS_DIR/claude-code-notify.sh" 2>/dev/null; then
    run_test "claude-code-notify.sh contains 'exit 0'" "ok"
else
    run_test "claude-code-notify.sh contains 'exit 0'" "missing exit 0"
fi

# Test: contains tmux send-keys pattern
if grep -q 'tmux send-keys' "$HOOKS_DIR/claude-code-notify.sh" 2>/dev/null; then
    run_test "claude-code-notify.sh contains tmux send-keys" "ok"
else
    run_test "claude-code-notify.sh contains tmux send-keys" "missing tmux send-keys"
fi

echo ""
echo "=== Testing gemini-cli-notify.sh ==="

# Test: file exists
if [ -f "$HOOKS_DIR/gemini-cli-notify.sh" ]; then
    run_test "gemini-cli-notify.sh exists" "ok"
else
    run_test "gemini-cli-notify.sh exists" "file not found"
fi

# Test: file is executable
if [ -x "$HOOKS_DIR/gemini-cli-notify.sh" ]; then
    run_test "gemini-cli-notify.sh is executable" "ok"
else
    run_test "gemini-cli-notify.sh is executable" "not executable"
fi

# Test: syntax valid
if bash -n "$HOOKS_DIR/gemini-cli-notify.sh" 2>/dev/null; then
    run_test "gemini-cli-notify.sh bash -n passes" "ok"
else
    run_test "gemini-cli-notify.sh bash -n passes" "syntax error"
fi

# Test: exits 0 when TMUX_PANE not set
if env -i PATH="$PATH" bash "$HOOKS_DIR/gemini-cli-notify.sh" <<< '{}' 2>/dev/null; then
    run_test "gemini-cli-notify.sh exits 0 without TMUX_PANE" "ok"
else
    run_test "gemini-cli-notify.sh exits 0 without TMUX_PANE" "non-zero exit"
fi

# Test: contains 'exit 0'
if grep -q 'exit 0' "$HOOKS_DIR/gemini-cli-notify.sh" 2>/dev/null; then
    run_test "gemini-cli-notify.sh contains 'exit 0'" "ok"
else
    run_test "gemini-cli-notify.sh contains 'exit 0'" "missing exit 0"
fi

# Test: contains tmux send-keys pattern
if grep -q 'tmux send-keys' "$HOOKS_DIR/gemini-cli-notify.sh" 2>/dev/null; then
    run_test "gemini-cli-notify.sh contains tmux send-keys" "ok"
else
    run_test "gemini-cli-notify.sh contains tmux send-keys" "missing tmux send-keys"
fi

echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
