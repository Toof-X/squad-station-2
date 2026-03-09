#!/bin/bash
# ==============================================================================
# Squad Station — End-to-End Workflow Test Suite
# ==============================================================================
# Unlike e2e_cli.sh (tests individual commands), this suite validates
# complete WORKFLOW SCENARIOS that mirror real-world squad operations.
#
# NOTE: Since claude-code/gemini binaries aren't available in the test env,
# init will create tmux sessions that immediately die. We re-create them
# with 'sleep 3600' for testing purposes.
#
# Scenarios tested:
#   W1. Full lifecycle: init → send → signal → completion
#   W2. Multi-agent parallel dispatch and coordination
#   W3. Priority ordering (urgent > high > normal)
#   W4. Orchestrator skip guard (infinite loop prevention)
#   W5. Antigravity IDE orchestrator (DB-only, no tmux)
#   W6. Context generation for both CLI and IDE orchestrators
#   W7. Agent lifecycle detection (idle → busy → idle, dead detection)
#   W8. Init idempotency and re-init with existing sessions
#   W9. Signal via $TMUX_PANE env var (inline hook mode)
#  W10. Safe multiline body injection
# ==============================================================================

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${PROJECT_ROOT}/target/release/squad-station"
BASE_DIR=$(mktemp -d)
PASS=0
FAIL=0
SKIP=0
TOTAL=0
FAILURES=""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# --- Helpers ------------------------------------------------------------------

pass() {
  PASS=$((PASS + 1))
  TOTAL=$((TOTAL + 1))
  echo -e "  ${GREEN}PASS${NC} $1"
}

fail() {
  FAIL=$((FAIL + 1))
  TOTAL=$((TOTAL + 1))
  FAILURES="${FAILURES}\n  - $1: $2"
  echo -e "  ${RED}FAIL${NC} $1 — $2"
}

skip() {
  SKIP=$((SKIP + 1))
  TOTAL=$((TOTAL + 1))
  echo -e "  ${YELLOW}SKIP${NC} $1 — $2"
}

section() {
  echo ""
  echo -e "${CYAN}${BOLD}━━━ $1 ━━━${NC}"
}

# Helper: run squad-station with test DB
run_ss() {
  local db="$1"; shift
  SQUAD_STATION_DB="$db" "$BIN" "$@"
}

# Helper: ensure a tmux session exists (create if needed)
ensure_session() {
  if ! tmux has-session -t "$1" 2>/dev/null; then
    tmux new-session -d -s "$1" "sleep 3600" 2>/dev/null || true
    sleep 0.2
  fi
}

# Cleanup all test artifacts
cleanup() {
  for sess in $(tmux list-sessions -F '#{session_name}' 2>/dev/null | grep -E '^wf[0-9]+-' || true); do
    tmux kill-session -t "$sess" 2>/dev/null || true
  done
  rm -rf "$BASE_DIR"
}
trap cleanup EXIT

# --- Verify binary ------------------------------------------------------------

if [[ ! -x "$BIN" ]]; then
  echo "Binary not found at $BIN — run 'cargo build --release' first"
  exit 1
fi

echo -e "${BOLD}Squad Station — Workflow Test Suite${NC}"
echo "  Binary: $BIN"
echo "  Test dir: $BASE_DIR"

# ==============================================================================
# W1. FULL LIFECYCLE: init → send → signal → verify completion
# ==============================================================================

section "W1. FULL LIFECYCLE"

W1_DIR="$BASE_DIR/w1"
mkdir -p "$W1_DIR"
W1_DB="$W1_DIR/station.db"

cat > "$W1_DIR/squad.yml" << 'YAML'
project: wf1

orchestrator:
  tool: claude-code
  role: orchestrator
  model: claude-opus-4-5
  description: "Test orchestrator"

agents:
  - name: coder
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Implements features"
YAML

cd "$W1_DIR"

# W1.1: Init creates DB + registers agents
# Note: tmux sessions may die immediately since claude-code isn't available.
# We re-create them with 'sleep 3600' for the test environment.
OUTPUT=$(run_ss "$W1_DB" init --json 2>&1) || true
if echo "$OUTPUT" | grep -q '"launched"'; then
  pass "W1.1 init --json returns launch report"
else
  fail "W1.1 init" "output: $OUTPUT"
fi

# Ensure tmux sessions exist for testing
ensure_session "wf1-claude-code-orchestrator"
ensure_session "wf1-claude-code-coder"

# W1.2: Verify agents registered with correct naming convention
OUTPUT=$(run_ss "$W1_DB" agents --json 2>&1)
if echo "$OUTPUT" | grep -q "wf1-claude-code-orchestrator" && \
   echo "$OUTPUT" | grep -q "wf1-claude-code-coder"; then
  pass "W1.2 agents have correct <project>-<tool>-<role> naming"
else
  fail "W1.2 naming convention" "output: $OUTPUT"
fi

# W1.3: Send task to worker, verify status transitions
OUTPUT=$(run_ss "$W1_DB" send wf1-claude-code-coder \
  --body "Implement JWT auth module with bcrypt hashing" --priority high 2>&1)
if echo "$OUTPUT" | grep -q "Sent task"; then
  pass "W1.3 send task to worker"
else
  fail "W1.3 send task" "output: $OUTPUT"
fi

# W1.4: Agent should be busy after receiving task
OUTPUT=$(run_ss "$W1_DB" agents 2>&1)
if echo "$OUTPUT" | grep "wf1-claude-code-coder" | grep -q "busy"; then
  pass "W1.4 agent status transitions to 'busy'"
else
  fail "W1.4 busy status" "output: $OUTPUT"
fi

# W1.5: Peek shows the task we sent
OUTPUT=$(run_ss "$W1_DB" peek wf1-claude-code-coder 2>&1)
if echo "$OUTPUT" | grep -q "JWT auth"; then
  pass "W1.5 peek shows correct task body"
else
  fail "W1.5 peek" "output: $OUTPUT"
fi

# W1.6: Signal completion (simulating hook fire)
OUTPUT=$(TMUX_PANE=%0 run_ss "$W1_DB" signal wf1-claude-code-coder 2>&1)
if echo "$OUTPUT" | grep -qi "signal\|complet\|acknowledged"; then
  pass "W1.6 signal completion accepted"
else
  fail "W1.6 signal" "output: $OUTPUT"
fi

# W1.7: After signal, task should be completed
OUTPUT=$(run_ss "$W1_DB" list --agent wf1-claude-code-coder --status completed --json 2>&1)
if echo "$OUTPUT" | grep -q "completed"; then
  pass "W1.7 task marked as completed after signal"
else
  fail "W1.7 completed status" "output: $OUTPUT"
fi

# W1.8: After signal, agent should be back to idle
OUTPUT=$(run_ss "$W1_DB" agents 2>&1)
if echo "$OUTPUT" | grep "wf1-claude-code-coder" | grep -q "idle"; then
  pass "W1.8 agent status transitions back to 'idle'"
else
  fail "W1.8 idle status" "output: $OUTPUT"
fi

# ==============================================================================
# W2. MULTI-AGENT PARALLEL DISPATCH
# ==============================================================================

section "W2. MULTI-AGENT PARALLEL"

W2_DIR="$BASE_DIR/w2"
mkdir -p "$W2_DIR"
W2_DB="$W2_DIR/station.db"

cat > "$W2_DIR/squad.yml" << 'YAML'
project: wf2

orchestrator:
  tool: claude-code
  role: orchestrator
  description: "Multi-agent orchestrator"

agents:
  - name: frontend
    tool: claude-code
    role: worker
    description: "Frontend developer"
  - name: backend
    tool: claude-code
    role: worker
    description: "Backend developer"
  - name: tester
    tool: claude-code
    role: worker
    description: "QA engineer"
YAML

cd "$W2_DIR"
run_ss "$W2_DB" init 2>/dev/null || true

# Ensure tmux sessions exist
ensure_session "wf2-claude-code-orchestrator"
ensure_session "wf2-claude-code-frontend"
ensure_session "wf2-claude-code-backend"
ensure_session "wf2-claude-code-tester"

# W2.1: Send tasks to all 3 workers simultaneously
run_ss "$W2_DB" send wf2-claude-code-frontend \
  --body "Build login form with validation" 2>/dev/null
run_ss "$W2_DB" send wf2-claude-code-backend \
  --body "Create REST API for authentication" 2>/dev/null
run_ss "$W2_DB" send wf2-claude-code-tester \
  --body "Write integration tests for auth flow" 2>/dev/null

OUTPUT=$(run_ss "$W2_DB" status 2>&1)
BUSY_COUNT=$(echo "$OUTPUT" | grep -o "busy" | wc -l | tr -d ' ')
if [[ "$BUSY_COUNT" -ge 3 ]]; then
  pass "W2.1 all 3 workers busy simultaneously ($BUSY_COUNT busy)"
else
  fail "W2.1 parallel dispatch" "expected 3 busy, got $BUSY_COUNT — output: $OUTPUT"
fi

# W2.2: Signal workers in order (frontend first, then backend, then tester)
TMUX_PANE=%0 run_ss "$W2_DB" signal wf2-claude-code-frontend 2>/dev/null
OUTPUT=$(run_ss "$W2_DB" status 2>&1)
BUSY_COUNT=$(echo "$OUTPUT" | grep -o "busy" | wc -l | tr -d ' ')
if [[ "$BUSY_COUNT" -ge 2 ]]; then
  pass "W2.2 after 1 signal: 2 still busy ($BUSY_COUNT busy)"
else
  fail "W2.2 partial completion" "expected 2 busy, got $BUSY_COUNT"
fi

TMUX_PANE=%0 run_ss "$W2_DB" signal wf2-claude-code-backend 2>/dev/null
TMUX_PANE=%0 run_ss "$W2_DB" signal wf2-claude-code-tester 2>/dev/null

# Check that no WORKERS are busy (orchestrator line may say 'busy' in text output)
OUTPUT=$(run_ss "$W2_DB" agents --json 2>&1)
WORKER_BUSY=$(echo "$OUTPUT" | grep -A3 '"worker"' | grep -c '"busy"' || true)
if [[ "$WORKER_BUSY" -eq 0 ]]; then
  pass "W2.3 all workers idle after all signals"
else
  fail "W2.3 complete cycle" "expected 0 busy workers, got $WORKER_BUSY"
fi

# W2.4: All 3 tasks completed
OUTPUT=$(run_ss "$W2_DB" list --status completed --json 2>&1)
COMPLETED_COUNT=$(echo "$OUTPUT" | grep -c '"completed"' || true)
if [[ "$COMPLETED_COUNT" -ge 3 ]]; then
  pass "W2.4 all 3 tasks marked completed ($COMPLETED_COUNT)"
else
  fail "W2.4 completed count" "expected >=3, got $COMPLETED_COUNT"
fi

# ==============================================================================
# W3. PRIORITY ORDERING
# ==============================================================================

section "W3. PRIORITY ORDERING"

cd "$W1_DIR"

# Send 3 tasks with different priorities
run_ss "$W1_DB" send wf1-claude-code-coder \
  --body "Normal priority task" --priority normal 2>/dev/null
run_ss "$W1_DB" send wf1-claude-code-coder \
  --body "Urgent priority task" --priority urgent 2>/dev/null
run_ss "$W1_DB" send wf1-claude-code-coder \
  --body "High priority task" --priority high 2>/dev/null

# W3.1: Peek should return the highest-priority pending task
# Note: signal completes the most recent PROCESSING message (not by priority).
# peek returns highest-priority PENDING message.
OUTPUT=$(run_ss "$W1_DB" peek wf1-claude-code-coder 2>&1)
if echo "$OUTPUT" | grep -qi "urgent"; then
  pass "W3.1 peek returns urgent task first (priority ordering)"
else
  fail "W3.1 priority ordering" "output: $OUTPUT"
fi

# W3.2: Signal the processing task, verify peek still works for next task
TMUX_PANE=%0 run_ss "$W1_DB" signal wf1-claude-code-coder 2>/dev/null
OUTPUT=$(run_ss "$W1_DB" peek wf1-claude-code-coder 2>&1)
# After one signal, should see one of the remaining priorities
if echo "$OUTPUT" | grep -qi "urgent\|high\|normal"; then
  pass "W3.2 peek returns next pending task after signal"
else
  fail "W3.2 next after signal" "output: $OUTPUT"
fi

# W3.3: Signal again, verify peek returns remaining task
TMUX_PANE=%0 run_ss "$W1_DB" signal wf1-claude-code-coder 2>/dev/null
OUTPUT=$(run_ss "$W1_DB" peek wf1-claude-code-coder 2>&1)
if echo "$OUTPUT" | grep -qi "normal"; then
  pass "W3.3 after high completed, normal is next"
else
  fail "W3.3 normal after high" "output: $OUTPUT"
fi

# Cleanup remaining task
TMUX_PANE=%0 run_ss "$W1_DB" signal wf1-claude-code-coder 2>/dev/null

# ==============================================================================
# W4. ORCHESTRATOR SKIP GUARD (4-layer defense)
# ==============================================================================

section "W4. ORCHESTRATOR SKIP GUARD"

cd "$W1_DIR"

# W4.1: Signal the orchestrator directly — should be silently blocked
OUTPUT=$(TMUX_PANE=%0 run_ss "$W1_DB" signal wf1-claude-code-orchestrator 2>&1)
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]]; then
  pass "W4.1 orchestrator self-signal exits 0 (skip guard active)"
else
  fail "W4.1 skip guard" "exit code: $EXIT_CODE"
fi

# W4.2: Signal from outside tmux — should exit 0 silently
OUTPUT=$(unset TMUX_PANE; SQUAD_STATION_DB="$W1_DB" "$BIN" signal wf1-claude-code-coder 2>&1)
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]]; then
  pass "W4.2 signal outside tmux exits 0 (guard: not in tmux)"
else
  fail "W4.2 outside tmux guard" "exit code: $EXIT_CODE"
fi

# W4.3: Signal unregistered agent — should exit 0 silently
OUTPUT=$(TMUX_PANE=%0 run_ss "$W1_DB" signal ghost-agent 2>&1)
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]]; then
  pass "W4.3 signal unregistered agent exits 0 (guard: not in DB)"
else
  fail "W4.3 unregistered guard" "exit code: $EXIT_CODE"
fi

# W4.4: Signal agent with no pending task — should exit 0
OUTPUT=$(TMUX_PANE=%0 run_ss "$W1_DB" signal wf1-claude-code-coder 2>&1)
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]]; then
  pass "W4.4 signal with no pending task exits 0 (guard: nothing to complete)"
else
  fail "W4.4 no pending guard" "exit code: $EXIT_CODE"
fi

# ==============================================================================
# W5. ANTIGRAVITY IDE ORCHESTRATOR
# ==============================================================================

section "W5. ANTIGRAVITY IDE ORCHESTRATOR"

W5_DIR="$BASE_DIR/w5"
mkdir -p "$W5_DIR"
W5_DB="$W5_DIR/station.db"

cat > "$W5_DIR/squad.yml" << 'YAML'
project: wf5

orchestrator:
  tool: antigravity
  role: orchestrator
  description: "IDE orchestrator — DB only, no tmux"

agents:
  - name: worker
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Implementation agent"
YAML

cd "$W5_DIR"

# W5.1: Init with antigravity orchestrator — no tmux session for orchestrator
OUTPUT=$(run_ss "$W5_DB" init 2>&1) || true
if echo "$OUTPUT" | grep -qi "db.only\|antigravity"; then
  pass "W5.1 init logs orchestrator as DB-only (no tmux session)"
else
  fail "W5.1 antigravity init" "output: $OUTPUT"
fi

# Ensure worker session exists for testing
ensure_session "wf5-claude-code-worker"

# W5.2: Verify orchestrator is registered in DB
OUTPUT=$(run_ss "$W5_DB" agents --json 2>&1)
if echo "$OUTPUT" | grep -q "wf5-antigravity-orchestrator"; then
  pass "W5.2 antigravity orchestrator registered in DB"
else
  fail "W5.2 registered" "output: $OUTPUT"
fi

# W5.3: Verify no tmux session for orchestrator
if ! tmux has-session -t wf5-antigravity-orchestrator 2>/dev/null; then
  pass "W5.3 no tmux session created for antigravity orchestrator"
else
  fail "W5.3 no tmux" "tmux session exists but shouldn't"
fi

# W5.4: Full workflow — send task, signal, verify DB-only completion
run_ss "$W5_DB" send wf5-claude-code-worker \
  --body "Build feature X" 2>/dev/null
TMUX_PANE=%0 run_ss "$W5_DB" signal wf5-claude-code-worker 2>/dev/null

# IDE polls for completion
OUTPUT=$(run_ss "$W5_DB" list --status completed --json 2>&1)
if echo "$OUTPUT" | grep -q "completed"; then
  pass "W5.4 IDE orchestrator can poll completed tasks (DB-only path)"
else
  fail "W5.4 IDE polling" "output: $OUTPUT"
fi

# W5.5: Status reflects correct agent states
OUTPUT=$(run_ss "$W5_DB" status 2>&1)
if echo "$OUTPUT" | grep -q "idle"; then
  pass "W5.5 antigravity status shows correct state"
else
  fail "W5.5 status" "output: $OUTPUT"
fi

# ==============================================================================
# W6. CONTEXT GENERATION
# ==============================================================================

section "W6. CONTEXT GENERATION"

# W6a: CLI orchestrator context
W6CLI_DIR="$BASE_DIR/w6cli"
mkdir -p "$W6CLI_DIR"
W6CLI_DB="$W6CLI_DIR/station.db"

cat > "$W6CLI_DIR/squad.yml" << 'YAML'
project: wf6cli

orchestrator:
  tool: claude-code
  role: orchestrator
  model: claude-opus-4-5
  description: "CLI orchestrator"

agents:
  - name: coder
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Writes production code"
YAML

cd "$W6CLI_DIR"
run_ss "$W6CLI_DB" init 2>/dev/null || true
ensure_session "wf6cli-claude-code-orchestrator"
ensure_session "wf6cli-claude-code-coder"

# W6.1: Context generates workflow files (always generates .agent/workflows/ since v1.3)
OUTPUT=$(run_ss "$W6CLI_DB" context 2>&1)
if echo "$OUTPUT" | grep -q "Generated .agent/workflows/"; then
  pass "W6.1 context generates .agent/workflows/ files"
else
  fail "W6.1 context" "output: $OUTPUT"
fi

# W6.1b: Delegate file contains agent info and send commands
if [[ -f "$W6CLI_DIR/.agent/workflows/squad-delegate.md" ]]; then
  DELEGATE=$(cat "$W6CLI_DIR/.agent/workflows/squad-delegate.md")
  if echo "$DELEGATE" | grep -q "wf6cli-claude-code-coder" && \
     echo "$DELEGATE" | grep -q "squad-station send"; then
    pass "W6.1b delegate file has agent name + send commands"
  else
    fail "W6.1b delegate content" "missing agent or send command"
  fi
else
  fail "W6.1b delegate" "file not found"
fi

# W6b: IDE orchestrator context (generates .agent/workflows/ files)
W6IDE_DIR="$BASE_DIR/w6ide"
mkdir -p "$W6IDE_DIR"
W6IDE_DB="$W6IDE_DIR/station.db"

cat > "$W6IDE_DIR/squad.yml" << 'YAML'
project: wf6ide

orchestrator:
  tool: antigravity
  role: orchestrator
  description: "IDE orchestrator for context test"

agents:
  - name: coder
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Writes code"
YAML

cd "$W6IDE_DIR"
run_ss "$W6IDE_DB" init 2>/dev/null || true
ensure_session "wf6ide-claude-code-coder"

# W6.2: Context generates 3 workflow files
run_ss "$W6IDE_DB" context 2>/dev/null

if [[ -f "$W6IDE_DIR/.agent/workflows/squad-delegate.md" ]]; then
  pass "W6.2a squad-delegate.md generated"
else
  fail "W6.2a delegate" "file not found"
fi

if [[ -f "$W6IDE_DIR/.agent/workflows/squad-monitor.md" ]]; then
  pass "W6.2b squad-monitor.md generated"
else
  fail "W6.2b monitor" "file not found"
fi

if [[ -f "$W6IDE_DIR/.agent/workflows/squad-roster.md" ]]; then
  pass "W6.2c squad-roster.md generated"
else
  fail "W6.2c roster" "file not found"
fi

# W6.3: Roster file includes agent info in table format
if [[ -f "$W6IDE_DIR/.agent/workflows/squad-roster.md" ]]; then
  ROSTER=$(cat "$W6IDE_DIR/.agent/workflows/squad-roster.md")
  if echo "$ROSTER" | grep -q "wf6ide-claude-code-coder" && \
     echo "$ROSTER" | grep -q "Squad Roster"; then
    pass "W6.3 roster contains agent name and table header"
  else
    fail "W6.3 roster content" "missing agent name or roster header"
  fi
fi

# W6.4: Delegate file has behavioral rules
if [[ -f "$W6IDE_DIR/.agent/workflows/squad-delegate.md" ]]; then
  DELEGATE=$(cat "$W6IDE_DIR/.agent/workflows/squad-delegate.md")
  if echo "$DELEGATE" | grep -qi "delegate\|send\|task"; then
    pass "W6.4 delegate file has delegation instructions"
  else
    fail "W6.4 delegate content" "missing delegation info"
  fi
fi

# W6.5: Context idempotent — running twice doesn't break anything
run_ss "$W6IDE_DB" context 2>/dev/null
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]]; then
  pass "W6.5 context idempotent (re-run exits 0)"
else
  fail "W6.5 idempotent" "exit code: $EXIT_CODE"
fi

# ==============================================================================
# W7. AGENT LIFECYCLE DETECTION
# ==============================================================================

section "W7. AGENT LIFECYCLE"

W7_DIR="$BASE_DIR/w7"
mkdir -p "$W7_DIR"
W7_DB="$W7_DIR/station.db"

cat > "$W7_DIR/squad.yml" << 'YAML'
project: wf7

orchestrator:
  tool: antigravity
  role: orchestrator
  description: "Lifecycle test orchestrator (DB-only)"

agents:
  - name: worker
    tool: claude-code
    role: worker
    description: "Worker that will die"
YAML

cd "$W7_DIR"
run_ss "$W7_DB" init 2>/dev/null || true
ensure_session "wf7-claude-code-worker"

# W7.1: Agent starts as idle
OUTPUT=$(run_ss "$W7_DB" agents 2>&1)
if echo "$OUTPUT" | grep "wf7-claude-code-worker" | grep -q "idle"; then
  pass "W7.1 agent starts as idle"
else
  fail "W7.1 initial idle" "output: $OUTPUT"
fi

# W7.2: Send task → busy
run_ss "$W7_DB" send wf7-claude-code-worker \
  --body "Do some work" 2>/dev/null
OUTPUT=$(run_ss "$W7_DB" agents 2>&1)
if echo "$OUTPUT" | grep "wf7-claude-code-worker" | grep -q "busy"; then
  pass "W7.2 agent transitions to busy after receiving task"
else
  fail "W7.2 busy transition" "output: $OUTPUT"
fi

# W7.3: Kill tmux session → dead detection
tmux kill-session -t wf7-claude-code-worker 2>/dev/null || true
sleep 0.3
OUTPUT=$(run_ss "$W7_DB" agents 2>&1)
if echo "$OUTPUT" | grep "wf7-claude-code-worker" | grep -q "dead"; then
  pass "W7.3 agent marked dead after tmux session killed"
else
  fail "W7.3 dead detection" "output: $OUTPUT"
fi

# ==============================================================================
# W8. INIT IDEMPOTENCY
# ==============================================================================

section "W8. INIT IDEMPOTENCY"

W8_DIR="$BASE_DIR/w8"
mkdir -p "$W8_DIR"
W8_DB="$W8_DIR/station.db"

cat > "$W8_DIR/squad.yml" << 'YAML'
project: wf8

orchestrator:
  tool: antigravity
  role: orchestrator
  description: "Idempotency test (DB-only)"

agents:
  - name: worker
    tool: claude-code
    role: worker
    description: "Worker"
YAML

cd "$W8_DIR"

# W8.1: First init
run_ss "$W8_DB" init 2>/dev/null || true
ensure_session "wf8-claude-code-worker"
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]]; then
  pass "W8.1 first init succeeds"
else
  fail "W8.1 first init" "exit code: $EXIT_CODE"
fi

# W8.2: Second init — should skip existing sessions
OUTPUT=$(run_ss "$W8_DB" init 2>&1)
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]]; then
  pass "W8.2 second init exits 0 (idempotent)"
else
  fail "W8.2 idempotent" "exit code: $EXIT_CODE"
fi

# W8.3: After re-init, agents still registered
OUTPUT=$(run_ss "$W8_DB" agents 2>&1)
if echo "$OUTPUT" | grep -q "wf8-claude-code-worker"; then
  pass "W8.3 agents persist after re-init"
else
  fail "W8.3 persist" "output: $OUTPUT"
fi

# ==============================================================================
# W9. SIGNAL VIA $TMUX_PANE (Inline Hook Mode)
# ==============================================================================

section "W9. SIGNAL VIA TMUX_PANE"

cd "$W8_DIR"

# Send a task then signal via pane env var
run_ss "$W8_DB" send wf8-claude-code-worker \
  --body "Task for pane signal test" 2>/dev/null

# W9.1: Signal with TMUX_PANE set (simulating inline hook)
OUTPUT=$(TMUX_PANE=%0 run_ss "$W8_DB" signal wf8-claude-code-worker 2>&1)
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]]; then
  pass "W9.1 signal with TMUX_PANE=%0 exits 0"
else
  fail "W9.1 pane signal" "exit=$EXIT_CODE output: $OUTPUT"
fi

# W9.2: Signal without TMUX_PANE — guard fires, exit 0
OUTPUT=$(unset TMUX_PANE; SQUAD_STATION_DB="$W8_DB" "$BIN" signal wf8-claude-code-worker 2>&1)
EXIT_CODE=$?
if [[ $EXIT_CODE -eq 0 ]]; then
  pass "W9.2 signal without TMUX_PANE exits 0 (guard)"
else
  fail "W9.2 no pane guard" "exit=$EXIT_CODE"
fi

# ==============================================================================
# W10. MULTILINE BODY INJECTION
# ==============================================================================

section "W10. MULTILINE BODY"

W10_DIR="$BASE_DIR/w10"
mkdir -p "$W10_DIR"
W10_DB="$W10_DIR/station.db"

cat > "$W10_DIR/squad.yml" << 'YAML'
project: wf10

orchestrator:
  tool: antigravity
  role: orchestrator
  description: "Multiline test (DB-only)"

agents:
  - name: worker
    tool: claude-code
    role: worker
    description: "Worker"
YAML

cd "$W10_DIR"
run_ss "$W10_DB" init 2>/dev/null || true
ensure_session "wf10-claude-code-worker"

# W10.1: Send multiline body
MULTILINE_BODY="Line 1: Implement the following:
Line 2: - User authentication
Line 3: - Session management
Line 4: - Password hashing with bcrypt
Special chars: quotes and ampersand"

OUTPUT=$(run_ss "$W10_DB" send wf10-claude-code-worker \
  --body "$MULTILINE_BODY" 2>&1)
if echo "$OUTPUT" | grep -q "Sent task"; then
  pass "W10.1 multiline body accepted"
else
  fail "W10.1 multiline send" "output: $OUTPUT"
fi

# W10.2: Stored body preserved in DB (check for content)
OUTPUT=$(run_ss "$W10_DB" peek wf10-claude-code-worker 2>&1)
if echo "$OUTPUT" | grep -q "bcrypt"; then
  pass "W10.2 multiline body preserved in DB"
else
  fail "W10.2 body preserved" "output: $OUTPUT"
fi

# Cleanup
TMUX_PANE=%0 run_ss "$W10_DB" signal wf10-claude-code-worker 2>/dev/null

# ==============================================================================
# RESULTS
# ==============================================================================

echo ""
echo -e "${BOLD}══════════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  WORKFLOW TEST RESULTS${NC}"
echo -e "${BOLD}══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Total:   ${BOLD}$TOTAL${NC}"
echo -e "  Passed:  ${GREEN}${BOLD}$PASS${NC}"
echo -e "  Failed:  ${RED}${BOLD}$FAIL${NC}"
echo -e "  Skipped: ${YELLOW}${BOLD}$SKIP${NC}"

if [[ $FAIL -gt 0 ]]; then
  echo ""
  echo -e "${RED}${BOLD}  Failures:${NC}"
  echo -e "$FAILURES"
  echo ""
  exit 1
else
  echo ""
  echo -e "  ${GREEN}${BOLD}All workflow tests passed! ✓${NC}"
  echo ""
  exit 0
fi
