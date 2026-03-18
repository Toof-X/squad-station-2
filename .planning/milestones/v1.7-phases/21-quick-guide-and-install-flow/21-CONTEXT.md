# Phase 21: Quick Guide and Install Flow - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Add a quick guide page (second state) to the welcome TUI state machine, and make both install paths (npm and curl) auto-launch `squad-station` in interactive terminals after a successful install. Quick guide content and navigation are self-contained within the existing welcome TUI. Install auto-launch is TTY-guarded in both `bin/run.js` (npm) and `install.sh` (curl).

</domain>

<decisions>
## Implementation Decisions

### Quick guide content
- Mental model: 1 orchestrator AI coordinates N worker agents via squad-station. Each agent runs in its own tmux session. Orchestrator sends tasks, agents signal completion.
- Format: concept summary (1-2 lines) + 3 numbered steps in plain English (no CLI commands on this page)
  1. Set up your squad (init)
  2. Send tasks to agents
  3. Agents signal completion automatically via hooks
- Tone: minimal, sparse — plain text with breathing room. No borders or boxes.
- Footer line: "Run squad-station --help for all commands" (consistent with existing static welcome screen)

### Guide page layout
- Full area given to guide content — no BigText title on the guide page
- Centered header line: "Quick Guide" (or similar)
- Blank line, then numbered steps, then blank line, then footer line
- Same hint bar area at the bottom (just different hint text)

### Guide navigation
- Key to open guide from title page: **Tab** or **Right arrow** — shown in hint bar as `Tab: Guide`
- On guide page, hint bar: `Tab/←: Back  Q: Quit` — no Enter action on the guide page
- Back key: same key (Tab or Left arrow) returns to title page
- Countdown behavior: **resets to 5s** when entering the guide page (user is actively reading)
- Title page hint bar updated to include: `Enter: [action]  Tab: Guide  Q: Quit  auto-exit Ns`

### Auto-launch after install
- Both install paths **exec the binary** in interactive terminals (REQUIREMENTS.md wins over research note)
- Guard condition: TTY check only
  - Shell (curl installer): `[ -t 1 ]` (stdout is a terminal)
  - Node (npm run.js): `process.stdout.isTTY === true`
- No additional CI env var or root guards — TTY check alone is sufficient
- Non-interactive environments (CI, pipes, sudo) degrade silently — no exec, no extra output

### npm auto-launch placement
- Auto-launch added at the bottom of the existing `install()` function in `npm-package/bin/run.js`
- No new postinstall.js file — install flow already lives in `npx squad-station install`
- Implementation: `spawnSync(destPath, [], { stdio: 'inherit' })` using the full `destPath` resolved during binary download (sidesteps any PATH uncertainty)
- TTY check: `if (process.stdout.isTTY) { spawnSync(destPath, [], { stdio: 'inherit' }); }`

### curl installer auto-launch
- Auto-launch added at end of `install.sh` after the success message
- Uses full install path: `exec "${INSTALL_DIR}/squad-station"`
- Guard: `if [ -t 1 ]; then exec "${INSTALL_DIR}/squad-station"; fi`
- `exec` replaces the shell process — clean handoff, no extra process in the tree

### Claude's Discretion
- Exact wording of the 3 numbered steps on the guide page
- Exact wording of the "Quick Guide" header line
- Left arrow key binding details (whether Left arrow is also accepted as a back key alongside Tab)
- Whether the guide page shows a page indicator (e.g., "1/2" or "● ○") in the hint bar

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing TUI implementation
- `src/commands/welcome.rs` — Current welcome TUI with WelcomeAction enum, routing_action(), hint_bar_text(), draw_welcome(), run_welcome_tui(). Guide page is a new state added to this module.
- `src/commands/ui.rs` — Reference for AlternateScreen + raw mode pattern (already mirrored in welcome.rs)

### Entry points for install scripts
- `npm-package/bin/run.js` — `install()` function: auto-launch goes at the bottom of this function after scaffoldProject() completes
- `install.sh` — curl installer: auto-launch goes after the final echo statements, before script exit

### Requirements
- `.planning/REQUIREMENTS.md` — WELCOME-05 (quick guide page), INSTALL-01, INSTALL-02, INSTALL-03 define acceptance criteria for this phase

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `welcome.rs` `routing_action()`: pure function dispatching on KeyCode → extend to handle Tab/Right arrow → new `ShowGuide` / `ShowTitle` WelcomeAction variants
- `welcome.rs` `hint_bar_text()`: extend or add `guide_hint_bar_text()` pure function for guide page hint
- `welcome.rs` `draw_welcome()`: guide page gets a parallel `draw_guide()` function using same Layout pattern
- `welcome.rs` `run_welcome_tui()`: event loop needs a `page: WelcomePage` state variable (Title | Guide) to dispatch to the right draw function

### Established Patterns
- WelcomeAction enum: already has LaunchInit, LaunchDashboard, Quit — add ShowGuide, ShowTitle variants (or handle page nav inline in the event loop)
- Pure function pattern (routing_action, hint_bar_text, commands_list): guide-related functions follow same testable-without-terminal pattern
- Countdown with `Instant::now() + Duration::from_secs(5)` and `saturating_duration_since`: reset deadline by reassigning when entering guide page

### Integration Points
- `run_welcome_tui()` event loop: add `page` state; draw different content per page; Tab key toggles page; reset deadline on page change
- `npm-package/bin/run.js` `install()`: `destPath` variable already holds full install path after download — use directly in spawnSync
- `install.sh`: `INSTALL_DIR` variable already set to the resolved install directory — use in exec guard

</code_context>

<specifics>
## Specific Ideas

- Guide page countdown resets to 5s on entry — same deadline variable reassigned with `Instant::now() + Duration::from_secs(5)` in the event loop
- curl installer uses `exec` (not a subshell call) to replace the script process cleanly
- npm uses `spawnSync(destPath, [], { stdio: 'inherit' })` with the full path already computed in `installBinary()` — no PATH lookup needed

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 21-quick-guide-and-install-flow*
*Context gathered: 2026-03-18*
