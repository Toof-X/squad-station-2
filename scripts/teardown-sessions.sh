#!/bin/bash
# teardown-sessions.sh — Kill all agent tmux sessions defined in squad.yml.
#
# Reads agents[] from squad.yml and kills matching tmux sessions.
# Sessions that don't exist are skipped.
#
# USAGE:
#   scripts/teardown-sessions.sh [squad.yml path]
#
# EXAMPLES:
#   scripts/teardown-sessions.sh
#   scripts/teardown-sessions.sh /path/to/squad.yml

set -euo pipefail

CONFIG="${1:-squad.yml}"

# ── Pre-flight ───────────────────────────────────────────────────────────────

if [ ! -f "$CONFIG" ]; then
  echo "Error: Config file not found: $CONFIG" >&2
  exit 1
fi

# ── Parse tmux-session and name from agents[] ────────────────────────────────

sessions=()
names=()
in_agents=false
current_tmux=""
current_name=""

flush_agent() {
  if [ -n "$current_tmux" ]; then
    sessions+=("$current_tmux")
    names+=("$current_name")
  fi
  current_tmux=""
  current_name=""
}

while IFS= read -r line; do
  if [[ "$line" =~ ^agents: ]]; then
    in_agents=true
    continue
  fi
  if $in_agents && [[ "$line" =~ ^[a-z] ]]; then
    flush_agent
    in_agents=false
    continue
  fi
  if ! $in_agents; then
    continue
  fi
  if [[ "$line" =~ ^[[:space:]]*-[[:space:]] ]]; then
    flush_agent
  fi
  if [[ "$line" =~ tmux-session: ]]; then
    current_tmux="$(echo "${line#*tmux-session:}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/^"//;s/"$//')"
  elif [[ "$line" =~ name: ]]; then
    current_name="$(echo "${line#*name:}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/^"//;s/"$//')"
  fi
done < "$CONFIG"
flush_agent

# ── Validate ─────────────────────────────────────────────────────────────────

if [ ${#sessions[@]} -eq 0 ]; then
  echo "No agents with tmux-session found in $CONFIG"
  exit 0
fi

# ── Print plan ───────────────────────────────────────────────────────────────

project="$(grep '^project:' "$CONFIG" | head -1 | sed 's/^project:[[:space:]]*//')"

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║         Squad Station  •  Session Teardown                   ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "  Project : ${project:-unknown}"
echo "  Config  : ${CONFIG}"
echo ""

# ── Kill sessions ────────────────────────────────────────────────────────────

killed=0
skipped=0

for i in "${!sessions[@]}"; do
  session="${sessions[$i]}"
  name="${names[$i]}"
  label="${name:-$session}"

  if tmux has-session -t "$session" 2>/dev/null; then
    tmux kill-session -t "$session"
    echo "  [KILLED] ${label} → session '${session}' terminated."
    killed=$((killed + 1))
  else
    echo "  [SKIP]   ${label} → session '${session}' not found."
    skipped=$((skipped + 1))
  fi
done

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo "  Killed: ${killed}, Skipped: ${skipped}"
echo ""
REMAINING=$(tmux list-sessions 2>/dev/null || true)
if [ -n "$REMAINING" ]; then
  echo "Remaining tmux sessions:"
  echo "$REMAINING" | sed 's/^/  /'
else
  echo "No tmux sessions remaining."
fi
echo ""
