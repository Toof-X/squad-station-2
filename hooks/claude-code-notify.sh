#!/bin/bash
# hooks/claude-code-notify.sh -- Forward permission prompt to orchestrator
# Registered under Notification event in .claude/settings.json
# matcher: "permission_prompt"
#
# Register in .claude/settings.json:
#   "hooks": { "Notification": [{ "matcher": "permission_prompt", "hooks": [{ "type": "command", "command": "/path/to/hooks/claude-code-notify.sh" }] }] }
#
# This script ALWAYS exits 0 -- non-zero exits may interfere with provider.

# Read stdin fully to avoid broken pipe signal to Claude Code
NOTIFICATION=$(cat)

# Guard: must be running inside tmux
if [ -z "$TMUX_PANE" ]; then
    exit 0
fi

# Detect agent name from current tmux session
AGENT_NAME=$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -n1)
if [ -z "$AGENT_NAME" ]; then
    exit 0
fi

# Guard: squad-station binary must be available
SQUAD_BIN="${SQUAD_STATION_BIN:-squad-station}"
if ! command -v "$SQUAD_BIN" > /dev/null 2>&1; then
    exit 0
fi

# Extract message field from the Notification JSON
MESSAGE=$(echo "$NOTIFICATION" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('message',''))" 2>/dev/null)
if [ -z "$MESSAGE" ]; then
    exit 0
fi

# Find orchestrator session name via squad-station agents --json
ORCH_NAME=$("$SQUAD_BIN" agents --json 2>/dev/null | python3 -c "import sys,json; agents=json.load(sys.stdin); orch=[a for a in agents if a.get('role')=='orchestrator']; print(orch[0]['name'] if orch else '')" 2>/dev/null)
if [ -z "$ORCH_NAME" ]; then
    exit 0
fi

# Guard: orchestrator tmux session must be alive
tmux has-session -t "$ORCH_NAME" 2>/dev/null || exit 0

# Forward notification to orchestrator session
tmux send-keys -l -t "$ORCH_NAME" "[NOTIFY] $AGENT_NAME needs permission: $MESSAGE"
tmux send-keys -t "$ORCH_NAME" "" Enter

exit 0
