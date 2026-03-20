#!/bin/bash
# setup-sessions.sh — Create tmux sessions for all agents defined in squad.yml.
#
# Reads agents[] from squad.yml and creates a tmux session for each one.
# Sessions that already exist are skipped.
#
# USAGE:
#   scripts/setup-sessions.sh [squad.yml path]
#
# EXAMPLES:
#   scripts/setup-sessions.sh
#   scripts/setup-sessions.sh /path/to/squad.yml

set -euo pipefail

# ── Load shared helpers ────────────────────────────────────────────────────
source "$(dirname "$0")/_common.sh"

CONFIG="${1:-squad.yml}"

# ── Pre-flight ───────────────────────────────────────────────────────────────

if [ ! -f "$CONFIG" ]; then
  echo "Error: Config file not found: $CONFIG" >&2
  exit 1
fi

if ! command -v tmux &>/dev/null; then
  echo "Error: tmux is not installed or not in PATH" >&2
  exit 1
fi

# ── Provider → launch command mapping ────────────────────────────────────────

provider_to_cmd() {
  local provider="$1"
  case "$provider" in
    claude-code) echo "claude" ;;
    gemini-cli)  echo "gemini" ;;
    *)           echo "$provider" ;;
  esac
}

# Build the full launch command from provider + model
build_launch_cmd() {
  local provider="$1"
  local model="$2"
  local base_cmd
  base_cmd=$(provider_to_cmd "$provider")

  local cmd="$base_cmd"

  # Provider-specific flags
  case "$provider" in
    claude-code)
      cmd="$cmd --dangerously-skip-permissions"
      ;;
    gemini-cli)
      cmd="$cmd --yolo"
      ;;
  esac

  # Append model if specified
  if [ -n "$model" ]; then
    cmd="$cmd --model $model"
  fi

  echo "$cmd"
}

# ── Parse agents from squad.yml ──────────────────────────────────────────────

# We parse the YAML line-by-line to extract agent entries.
# Each agent needs: tmux-session, provider, model (optional), name (for display).

agents_tmux=()
agents_provider=()
agents_model=()
agents_name=()

in_agents=false
current_tmux=""
current_provider=""
current_model=""
current_name=""

flush_agent() {
  if [ -n "$current_tmux" ] && [ -n "$current_provider" ]; then
    agents_tmux+=("$current_tmux")
    agents_provider+=("$current_provider")
    agents_model+=("$current_model")
    agents_name+=("$current_name")
  fi
  current_tmux=""
  current_provider=""
  current_model=""
  current_name=""
}

while IFS= read -r line; do
  # Detect agents: block
  if [[ "$line" =~ ^agents: ]]; then
    in_agents=true
    continue
  fi

  # Exit agents block when hitting a non-indented top-level key
  if $in_agents && [[ "$line" =~ ^[a-z] ]]; then
    flush_agent
    in_agents=false
    continue
  fi

  if ! $in_agents; then
    continue
  fi

  # New agent entry (- name: ...)
  if [[ "$line" =~ ^[[:space:]]*-[[:space:]] ]]; then
    flush_agent
  fi

  # Extract fields (strip quotes and whitespace)
  if [[ "$line" =~ tmux-session: ]]; then
    current_tmux="$(echo "${line#*tmux-session:}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/^"//;s/"$//')"
  elif [[ "$line" =~ provider: ]]; then
    current_provider="$(echo "${line#*provider:}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/^"//;s/"$//')"
  elif [[ "$line" =~ model: ]]; then
    current_model="$(echo "${line#*model:}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/^"//;s/"$//')"
  elif [[ "$line" =~ name: ]]; then
    current_name="$(echo "${line#*name:}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/^"//;s/"$//')"
  fi
done < "$CONFIG"

# Flush last agent
flush_agent

# ── Validate ─────────────────────────────────────────────────────────────────

if [ ${#agents_tmux[@]} -eq 0 ]; then
  echo "No agents with tmux-session found in $CONFIG"
  exit 0
fi

# ── Create sessions ──────────────────────────────────────────────────────────

# Extract project name for display
project="$(grep '^project:' "$CONFIG" | head -1 | sed 's/^project:[[:space:]]*//')"

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║         Squad Station  •  Session Setup                      ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "  Project : ${project:-unknown}"
echo "  Config  : ${CONFIG}"
echo "  Agents  : ${#agents_tmux[@]}"
echo ""

# ── Validate providers and models ────────────────────────────────────────────

validation_errors=0
for i in "${!agents_tmux[@]}"; do
  name="${agents_name[$i]}"
  provider="${agents_provider[$i]}"
  model="${agents_model[$i]}"
  if ! validate_provider_model "$name" "$provider" "$model"; then
    validation_errors=$((validation_errors + 1))
  fi
done

if [ "$validation_errors" -gt 0 ]; then
  echo ""
  echo "Aborted: ${validation_errors} validation error(s). Fix squad.yml before retrying." >&2
  exit 1
fi

# ── Create sessions ──────────────────────────────────────────────────────────

created=0
skipped=0

for i in "${!agents_tmux[@]}"; do
  session="${agents_tmux[$i]}"
  provider="${agents_provider[$i]}"
  model="${agents_model[$i]}"
  name="${agents_name[$i]}"
  launch_cmd=$(build_launch_cmd "$provider" "$model")

  label="${name} (${provider}${model:+, $model})"

  if tmux has-session -t "$session" 2>/dev/null; then
    echo "  [SKIP]   ${label} → session '${session}' already exists."
    skipped=$((skipped + 1))
  else
    tmux new-session -d -s "$session" -x 220 -y 50
    sleep 0.5
    tmux send-keys -t "${session}:0.0" "$launch_cmd" C-m
    echo "  [CREATE] ${label} → session '${session}' created."
    created=$((created + 1))
  fi
done

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo "  Created: ${created}, Skipped: ${skipped}"
echo ""
echo "Active tmux sessions:"
tmux list-sessions 2>/dev/null | sed 's/^/  /'
echo ""
echo "To attach:"
for session in "${agents_tmux[@]}"; do
  echo "  tmux attach -t ${session}"
done
echo ""
