#!/bin/bash
# DEPRECATED: This script is no longer required as of squad-station v1.3.
# Use the inline hook command instead (register directly in settings.json):
#
#   squad-station signal $TMUX_PANE
#
# This file is kept for reference only. It remains functional for users on older setups.
#
# hooks/claude-code.sh -- Signal squad-station when Claude Code finishes a response
# Registered under Stop event in .claude/settings.json or ~/.claude/settings.json
#
# Claude Code passes JSON via stdin (we discard it -- we only need the tmux session name)
# All guard logic (not-in-tmux, unregistered agent, orchestrator skip) is in the Rust binary.
# This script ALWAYS exits 0 -- exit 2 would prevent Claude from stopping (catastrophic).

# Drain stdin to avoid broken pipe signal to Claude Code
cat > /dev/null

# Detect agent name from current tmux session
if [ -z "$TMUX_PANE" ]; then
    exit 0  # Not in tmux -- not a managed agent
fi

AGENT_NAME=$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -n1)
if [ -z "$AGENT_NAME" ]; then
    exit 0  # Cannot determine session name
fi

# Delegate all guards and signal logic to the binary
SQUAD_BIN="${SQUAD_STATION_BIN:-squad-station}"
if ! command -v "$SQUAD_BIN" > /dev/null 2>&1; then
    echo "squad-station: warning: binary not found at '$SQUAD_BIN'" >&2
    exit 0
fi

"$SQUAD_BIN" signal "$AGENT_NAME" 2>&1 | (grep -i "warning\|error" >&2 || true)
exit 0
