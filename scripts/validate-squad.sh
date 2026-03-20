#!/bin/bash
# validate-squad.sh — Validate tmux sessions and SDD playbook paths from squad.yml.
#
# Checks:
#   1. All agent tmux-session values have a live tmux session
#   2. All sdd[].playbook paths exist and are readable
#
# USAGE:
#   scripts/validate-squad.sh [squad.yml path]
#
# EXIT CODES:
#   0 — all checks passed
#   1 — one or more checks failed

set -euo pipefail

# ── Load shared helpers ────────────────────────────────────────────────────
source "$(dirname "$0")/_common.sh"

CONFIG="${1:-squad.yml}"
ERRORS=0

# ── Helpers ──────────────────────────────────────────────────────────────────

red()   { printf '\033[0;31m%s\033[0m\n' "$1"; }
green() { printf '\033[0;32m%s\033[0m\n' "$1"; }
dim()   { printf '\033[0;90m%s\033[0m\n' "$1"; }

pass() { green "  [PASS] $1"; }
fail() { red   "  [FAIL] $1"; ERRORS=$((ERRORS + 1)); }

# ── Pre-flight ───────────────────────────────────────────────────────────────

if [ ! -f "$CONFIG" ]; then
  red "Config file not found: $CONFIG"
  exit 1
fi

# Check dependencies
for cmd in tmux; do
  if ! command -v "$cmd" &>/dev/null; then
    fail "$cmd is not installed or not in PATH"
  fi
done

echo ""
echo "Validating squad config: $CONFIG"
echo ""

# ── 1. Validate SDD playbook paths ──────────────────────────────────────────

echo "── SDD Playbooks ──"

# Extract playbook paths from sdd[].playbook using grep + sed (no yq dependency)
playbook_paths=()
in_sdd=false
while IFS= read -r line; do
  # Detect sdd: block
  if [[ "$line" =~ ^sdd: ]]; then
    in_sdd=true
    continue
  fi
  # Exit sdd block when hitting a non-indented key
  if $in_sdd && [[ "$line" =~ ^[a-z] ]]; then
    in_sdd=false
    continue
  fi
  # Extract playbook value
  if $in_sdd && [[ "$line" =~ playbook: ]]; then
    path="${line#*playbook:}"
    # Strip leading/trailing whitespace and quotes
    path="$(echo "$path" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/^"//;s/"$//;s/^'"'"'//;s/'"'"'$//')"
    playbook_paths+=("$path")
  fi
done < "$CONFIG"

if [ ${#playbook_paths[@]} -eq 0 ]; then
  dim "  No SDD playbooks declared in $CONFIG"
else
  for p in "${playbook_paths[@]}"; do
    if [ -f "$p" ] && [ -r "$p" ]; then
      pass "$p"
    else
      fail "$p — file not found or not readable"
    fi
  done
fi

echo ""

# ── 2. Validate agent providers and models ───────────────────────────────────

echo "── Agent Providers & Models ──"

# Parse agents[] to extract name, provider, model, tmux-session
agent_names=()
agent_providers=()
agent_models=()
agent_sessions=()

in_agents=false
cur_name=""
cur_provider=""
cur_model=""
cur_session=""

flush_agent() {
  if [ -n "$cur_provider" ]; then
    agent_names+=("$cur_name")
    agent_providers+=("$cur_provider")
    agent_models+=("$cur_model")
    agent_sessions+=("$cur_session")
  fi
  cur_name=""
  cur_provider=""
  cur_model=""
  cur_session=""
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
  strip() { echo "$1" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//;s/^"//;s/"$//'; }
  if [[ "$line" =~ name: ]]; then
    cur_name="$(strip "${line#*name:}")"
  elif [[ "$line" =~ provider: ]]; then
    cur_provider="$(strip "${line#*provider:}")"
  elif [[ "$line" =~ model: ]]; then
    cur_model="$(strip "${line#*model:}")"
  elif [[ "$line" =~ tmux-session: ]]; then
    cur_session="$(strip "${line#*tmux-session:}")"
  fi
done < "$CONFIG"
flush_agent

if [ ${#agent_providers[@]} -eq 0 ]; then
  dim "  No agents found in $CONFIG"
else
  for i in "${!agent_providers[@]}"; do
    name="${agent_names[$i]}"
    provider="${agent_providers[$i]}"
    model="${agent_models[$i]}"
    label="${name} (${provider}${model:+, $model})"

    if validate_provider_model "$name" "$provider" "$model" 2>/dev/null; then
      pass "$label"
    else
      valid_models=$(valid_models_for "$provider")
      if ! echo "$VALID_PROVIDERS" | grep -qw "$provider"; then
        fail "${name}: invalid provider '${provider}'. Valid: ${VALID_PROVIDERS}"
      else
        fail "${name}: invalid model '${model}' for '${provider}'. Valid: ${valid_models}"
      fi
    fi
  done
fi

echo ""

# ── 3. Validate tmux sessions ───────────────────────────────────────────────

echo "── Tmux Sessions ──"

# Collect active tmux sessions
active_sessions=""
if tmux list-sessions -F '#{session_name}' 2>/dev/null; then
  active_sessions=$(tmux list-sessions -F '#{session_name}' 2>/dev/null)
fi

if [ ${#agent_sessions[@]} -eq 0 ]; then
  dim "  No tmux-session fields found in agents[]"
else
  for s in "${agent_sessions[@]}"; do
    if [ -z "$s" ]; then
      continue
    fi
    if echo "$active_sessions" | grep -qx "$s"; then
      pass "tmux session '$s' is running"
    else
      fail "tmux session '$s' is NOT running"
    fi
  done
fi

echo ""

# ── Summary ──────────────────────────────────────────────────────────────────

if [ "$ERRORS" -gt 0 ]; then
  red "Validation failed: $ERRORS error(s)"
  exit 1
else
  green "All checks passed."
  exit 0
fi
