#!/bin/bash
# _common.sh — Shared helpers for squad station scripts.
#
# SOURCE this file; do not execute it directly.
#   source "$(dirname "$0")/_common.sh"

# ── Valid providers and models ───────────────────────────────────────────────
# Single source of truth for provider → model validation.

VALID_PROVIDERS="claude-code gemini-cli antigravity"

valid_models_for() {
  local provider="$1"
  case "$provider" in
    claude-code) echo "opus sonnet haiku" ;;
    gemini-cli)  echo "gemini-3.1-pro-preview gemini-3-flash-preview gemini-2.5-pro gemini-2.5-flash" ;;
    *)           echo "" ;;  # no model validation for other providers
  esac
}

# Validate provider + model. Prints error message and returns 1 on failure.
#   $1 — agent label (for error messages)
#   $2 — provider
#   $3 — model (can be empty)
validate_provider_model() {
  local label="$1"
  local provider="$2"
  local model="$3"

  # Check provider
  if ! echo "$VALID_PROVIDERS" | grep -qw "$provider"; then
    echo "  [FAIL] ${label}: invalid provider '${provider}'. Valid: ${VALID_PROVIDERS}" >&2
    return 1
  fi

  # Check model (only if specified and provider has a known model list)
  if [ -n "$model" ]; then
    local valid_models
    valid_models=$(valid_models_for "$provider")
    if [ -n "$valid_models" ] && ! echo "$valid_models" | grep -qw "$model"; then
      echo "  [FAIL] ${label}: invalid model '${model}' for provider '${provider}'. Valid: ${valid_models}" >&2
      return 1
    fi
  fi

  return 0
}

# ── sanitize_for_tmux ──────────────────────────────────────────────────────
# Make a string safe for use as a tmux session name.
#   • '.'  → tmux uses it as session.window.pane separator
#   • ':'  → tmux uses it as session:window separator
#   • ' '  → causes quoting headaches in targets
# Replace all of them with underscores.
sanitize_for_tmux() {
  local name="$1"
  name="${name//./_}"
  name="${name//:/_}"
  name="${name// /_}"
  echo "$name"
}

# ── resolve_folder ─────────────────────────────────────────────────────────
# Determine the project folder name (already sanitised for tmux).
#   $1 — explicit folder name (optional; falls back to git root or $PWD)
resolve_folder() {
  local folder
  if [ -n "$1" ]; then
    folder="$1"
  else
    local git_root
    git_root=$(git rev-parse --show-toplevel 2>/dev/null || echo "")
    if [ -n "$git_root" ]; then
      folder=$(basename "$git_root")
    else
      folder=$(basename "$PWD")
    fi
  fi
  sanitize_for_tmux "$folder"
}

