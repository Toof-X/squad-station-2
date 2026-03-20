#!/bin/bash
# DEPRECATED: This script is no longer required as of squad-station v1.3.
# Use the inline hook command instead (register directly in settings.json):
#
#   squad-station signal $TMUX_PANE
#
# This file is kept for reference only. It remains functional for users on older setups.
#
# hooks/gemini-cli.sh -- Signal squad-station when Gemini CLI finishes a response
# Registered under AfterAgent event in .gemini/settings.json
#
# Gemini CLI passes JSON via stdin (we discard it -- we only need the tmux session name)
# All guard logic is in the Rust binary. This script ALWAYS exits 0.
# Exit 2 would trigger Gemini CLI automatic retry -- equally catastrophic.

# Drain stdin to avoid broken pipe signal to Gemini CLI
cat > /dev/null

# Detect agent name from current tmux session
if [ -z "$TMUX_PANE" ]; then
    exit 0
fi

AGENT_NAME=$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -n1)
if [ -z "$AGENT_NAME" ]; then
    exit 0
fi

SQUAD_BIN="${SQUAD_STATION_BIN:-squad-station}"
if ! command -v "$SQUAD_BIN" > /dev/null 2>&1; then
    echo "squad-station: warning: binary not found at '$SQUAD_BIN'" >&2
    exit 0
fi

"$SQUAD_BIN" signal "$AGENT_NAME" 2>&1 | (grep -i "warning\|error" >&2 || true)
exit 0
