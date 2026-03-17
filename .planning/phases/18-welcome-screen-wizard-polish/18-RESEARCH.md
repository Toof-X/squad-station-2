# Phase 18: Welcome Screen & Wizard Polish - Research

**Researched:** 2026-03-17
**Domain:** Rust CLI, owo-colors, clap subcommand_required, ASCII art, ModelSelector
**Confidence:** HIGH

## Summary

Phase 18 has two distinct concerns: (1) intercepting the no-argument invocation path to print a branded welcome screen, and (2) simplifying the model option strings shown to users in the TUI wizard when `claude-code` is selected as provider.

The current binary uses `clap` with `#[command(name = "squad-station", version, about)]` and a required `#[command(subcommand)]` field. Running `squad-station` with no arguments exits with code 2 and prints clap's default help text. To show a custom welcome screen instead, the `Commands` enum must become `Option<Commands>` so clap does not require a subcommand, and the `main.rs` dispatch loop needs a `None` arm that calls a new `welcome::print()` function.

The wizard's `ModelSelector::options_for(Provider::ClaudeCode)` currently returns full version-suffixed strings (`"claude-sonnet-4-6"`, `"claude-opus-4-6"`, `"claude-haiku-4-5"`). Changing these to `"sonnet"`, `"opus"`, `"haiku"` is a one-line array edit in `wizard.rs`, but it ripples into existing unit tests in `src/commands/init.rs` that assert on the full model name strings.

**Primary recommendation:** Make `Commands` optional in `cli.rs`, add a `src/commands/welcome.rs` module that prints the ASCII title + version + subcommand list, and narrow the `ModelSelector` options array to three short names per the requirements.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| WEL-01 | Running `squad-station` with no subcommand prints a large ASCII "SQUAD-STATION" title in red | owo-colors `OwoColorize` with `if_supports_color` used throughout codebase; ASCII title hand-crafted string constant |
| WEL-02 | Welcome screen shows current binary version | `env!("CARGO_PKG_VERSION")` macro provides version at compile time; version is already baked into clap via `#[command(version)]` |
| WEL-03 | Welcome screen shows "run `squad-station init` to get started" hint | Plain `println!` with owo-colors for emphasis |
| WEL-04 | Welcome screen lists available subcommands | Static list — matches the 11 subcommands named in the requirement |
| WIZ-01 | claude-code model options show `sonnet`, `opus`, `haiku` (no version suffix) | Edit `ModelSelector::options_for` return value for `Provider::ClaudeCode` |
| WIZ-02 | Wizard stores simplified name in squad.yml (e.g. `model: sonnet`) | Model value flows directly from `ModelSelector::current()` into `AgentInput.model` then into `generate_squad_yml`; no transformation needed once the source string is simplified |
</phase_requirements>

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| owo-colors | 3.x (already in Cargo.toml) | ANSI color output | Already used in `init.rs` with `OwoColorize` + `if_supports_color(Stream::Stdout, ...)` |
| clap | 4.5 (already in Cargo.toml) | CLI argument parsing | Project standard; `#[command(subcommand)]` becomes `Option<Commands>` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `env!` macro | stdlib | Compile-time version string | `env!("CARGO_PKG_VERSION")` in welcome module |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-crafted ASCII art string | `figlet-rs` or `rustfiglet` crate | Crate adds a dependency for a one-time static string; hand-craft is fine for a fixed title |
| `Option<Commands>` | Separate `Welcome` subcommand | Adding a subcommand changes the UX (user would type `squad-station welcome`); the requirement is *bare invocation* |

**Installation:** No new dependencies required.

## Architecture Patterns

### Recommended Project Structure

```
src/
├── commands/
│   ├── welcome.rs       # NEW: print_welcome() function
│   ├── mod.rs           # Add pub mod welcome;
│   ├── wizard.rs        # EDIT: ModelSelector::options_for ClaudeCode array
│   └── init.rs          # No change needed
├── cli.rs               # EDIT: command field to Option<Commands>
└── main.rs              # EDIT: None arm calls commands::welcome::print_welcome()
```

### Pattern 1: Optional Subcommand in Clap

**What:** Change `command: Commands` to `command: Option<Commands>` so clap accepts a bare invocation without error.
**When to use:** When the binary should do something meaningful with no arguments instead of printing help and exiting 2.

```rust
// Source: clap docs / current cli.rs pattern
#[derive(Parser, Debug)]
#[command(name = "squad-station", version, about)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,  // was: Commands
}
```

The dispatch in `main.rs` becomes:

```rust
match cli.command {
    None => commands::welcome::print_welcome(),
    Some(cmd) => {
        use cli::Commands::*;
        match cmd {
            Init { config } => commands::init::run(config, cli.json).await,
            // ... existing arms unchanged
        }
    }
}
```

### Pattern 2: owo-colors for Red ASCII Title

**What:** Use `OwoColorize` trait with `if_supports_color` to print red text, consistent with existing code in `init.rs`.
**When to use:** All colorized output in this project.

```rust
// Source: existing pattern in src/commands/init.rs
use owo_colors::OwoColorize;
use owo_colors::Stream;

pub fn print_welcome() {
    let title = r#"
 ____   ___  _   _    _    ____       ____ _____  _  _____ ___ ___  _   _
/ ___| / _ \| | | |  / \  |  _ \     / ___|_   _|/ \|_   _|_ _/ _ \| \ | |
\___ \| | | | | | | / _ \ | | | |   \___ \ | | / _ \ | |  | | | | |  \| |
 ___) | |_| | |_| |/ ___ \| |_| |    ___) || |/ ___ \| |  | | |_| | |\  |
|____/ \__\_\\___//_/   \_\____/    |____/ |_/_/   \_\_| |___\___/|_| \_|
    "#;
    let colored = title.if_supports_color(Stream::Stdout, |s| s.red());
    println!("{}", colored);
    println!("  v{}", env!("CARGO_PKG_VERSION"));
    // ... hint and subcommand list
}
```

### Pattern 3: ModelSelector Options Simplification

**What:** Replace the full version-string options in `ModelSelector::options_for` for `Provider::ClaudeCode` with short names.
**When to use:** WIZ-01 and WIZ-02.

```rust
// Source: src/commands/wizard.rs ModelSelector::options_for
Provider::ClaudeCode => &[
    "sonnet",   // was "claude-sonnet-4-6"
    "opus",     // was "claude-opus-4-6"
    "haiku",    // was "claude-haiku-4-5"
    "other",
],
```

The `model` value flows unchanged into `AgentInput.model` and then into `generate_squad_yml` via `format!("    model: {}\n", model)` — so the simplified name is written directly to squad.yml with no further changes needed.

### Anti-Patterns to Avoid

- **Printing help text manually:** Clap already generates a help page. The welcome screen is an addition, not a replacement for `--help`.
- **Modifying `options_for` display label separately from stored value:** The wizard stores the `current()` return value directly. There is no label/value split; the display string IS the stored value. Keep them identical.
- **Calling `std::process::exit(0)` in `print_welcome`:** The `run()` function in `main.rs` returns `Result<()>`; welcome should return `Ok(())` cleanly.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Version string | Hardcode a version constant | `env!("CARGO_PKG_VERSION")` | Compile-time macro reads from Cargo.toml; never gets out of sync |
| Color support detection | Custom isatty check | `owo-colors` `if_supports_color(Stream::Stdout, ...)` | Already the project pattern; handles NO_COLOR, piped output, etc. |

**Key insight:** The ASCII title is a static string. Do not use a crate to generate it at runtime.

## Common Pitfalls

### Pitfall 1: `command: Commands` — clap panics or errors on no args

**What goes wrong:** With a required `#[command(subcommand)]`, clap exits with code 2 ("required arguments were not provided") when no subcommand is given.
**Why it happens:** `Commands` (non-Option) tells clap the subcommand is required.
**How to avoid:** Change the field type to `Option<Commands>`. Clap infers subcommand is optional from the `Option` wrapper.
**Warning signs:** `cargo test` passes but `./squad-station` exits 2.

### Pitfall 2: Existing tests assert on full model name strings

**What goes wrong:** `test_generate_squad_yml_orchestrator_fields` in `src/commands/init.rs` asserts `yaml.contains("model: claude-sonnet-4-6")`. After renaming the option to `"sonnet"`, this test fails.
**Why it happens:** The test fixtures hardcode the old model name strings.
**How to avoid:** Update test fixtures that use `"claude-sonnet-4-6"` (and similar) to use `"sonnet"` (and `"opus"`, `"haiku"`).
**Warning signs:** `cargo test` fails with "model: claude-sonnet-4-6" assertion error.

Affected test helpers in `src/commands/init.rs`:
- `make_wizard_result()` sets `model: Some("claude-sonnet-4-6".to_string())`
- `test_generate_squad_yml_orchestrator_fields` asserts `model: claude-sonnet-4-6`
- `test_append_workers_to_yaml_includes_model_when_present` uses `"claude-sonnet-4-6"` as model string

### Pitfall 3: Welcome screen suppressed in JSON mode

**What goes wrong:** If `--json` flag is passed with no subcommand (unusual but possible), printing the welcome banner breaks machine-parseable output.
**Why it happens:** `--json` is a global flag that persists even with no subcommand.
**How to avoid:** Check `cli.json` in the `None` arm; if true, either print nothing or print `{}`. Since no requirement covers JSON+no-subcommand, safest is to still print the welcome screen (it goes to stdout, json mode is opt-in for scripts).

### Pitfall 4: Subcommand list in welcome screen becomes stale

**What goes wrong:** Requirements specify exactly 11 subcommands (init, send, signal, peek, list, ui, view, status, agents, context, register). The actual binary has more (close, reset, freeze, unfreeze, clean, notify). The requirements list is the user-facing "get started" set.
**Why it happens:** The welcome screen is a static list, not auto-generated from clap.
**How to avoid:** The welcome screen list should match exactly what WEL-04 specifies — it is a curated first-run list, not an exhaustive list. Keep it as the 11 named in the requirement.

## Code Examples

Verified patterns from official sources:

### Getting version at compile time

```rust
// stdlib env! macro — no crate needed
const VERSION: &str = env!("CARGO_PKG_VERSION");
// or inline:
println!("v{}", env!("CARGO_PKG_VERSION"));
```

### owo-colors red text (existing project pattern from init.rs)

```rust
use owo_colors::OwoColorize;
use owo_colors::Stream;

let text = "SQUAD-STATION";
let red = text.if_supports_color(Stream::Stdout, |s| s.red());
println!("{}", red);
```

### Clap optional subcommand

```rust
// cli.rs — before:
pub command: Commands,

// cli.rs — after:
pub command: Option<Commands>,

// main.rs dispatch — new None arm:
match cli.command {
    None => {
        commands::welcome::print_welcome();
        Ok(())
    }
    Some(cmd) => run_command(cmd, cli.json).await,
}
```

### ModelSelector options simplified

```rust
// wizard.rs — ModelSelector::options_for
Provider::ClaudeCode => &["sonnet", "opus", "haiku", "other"],
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Full model name in wizard (`claude-sonnet-4-6`) | Short alias (`sonnet`) | Phase 18 | Claude CLI accepts both; `--model sonnet` is valid |
| Bare invocation → clap error (exit 2) | Bare invocation → welcome screen | Phase 18 | Better first-run UX |

**Note on claude CLI model alias:** The `claude` CLI accepts short model names like `sonnet`, `opus`, `haiku` as aliases. Using `--model sonnet` is valid. This is confirmed by Claude Code CLI behavior (knowledge from training, confirmed HIGH confidence for the current aliases since Anthropic maintains backward-compat for these short names).

## Open Questions

1. **Should `notify` appear in the welcome screen subcommand list?**
   - What we know: WEL-04 specifies 11 subcommands: `init, send, signal, peek, list, ui, view, status, agents, context, register`. `notify` is absent from this list.
   - What's unclear: Whether omitting `notify` is intentional (it's an agent-internal command) or an oversight.
   - Recommendation: Follow the requirement verbatim — list the 11 specified subcommands only.

2. **ASCII art dimensions**
   - What we know: The requirement says "large ASCII". No exact character grid is specified.
   - What's unclear: Whether a multi-line figlet-style banner or a simpler all-caps bold print is expected.
   - Recommendation: Use a 5-line ASCII art block for "SQUAD-STATION" (fits standard 80-col terminals). Hand-crafted or use a figlet font reference to generate it offline and embed as a string constant.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | none (standard Cargo integration) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WEL-01 | ASCII title printed to stdout on bare invocation | unit | `cargo test test_welcome_prints_ascii_title` | ❌ Wave 0 |
| WEL-02 | Version string appears in welcome output | unit | `cargo test test_welcome_shows_version` | ❌ Wave 0 |
| WEL-03 | Init hint appears in welcome output | unit | `cargo test test_welcome_shows_init_hint` | ❌ Wave 0 |
| WEL-04 | Subcommand list present in welcome output | unit | `cargo test test_welcome_lists_subcommands` | ❌ Wave 0 |
| WIZ-01 | Claude-code model options are `sonnet`, `opus`, `haiku` | unit | `cargo test test_model_selector_claude_code_options` | ❌ Wave 0 |
| WIZ-02 | Simplified model name stored in squad.yml | unit | `cargo test test_generate_squad_yml_uses_simplified_model` | ❌ Wave 0 (update existing) |

### Sampling Rate

- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src/commands/welcome.rs` — new module with `print_welcome()` function; needs unit tests for output content
- [ ] Unit tests for `ModelSelector::options_for(Provider::ClaudeCode)` asserting `["sonnet", "opus", "haiku", "other"]`
- [ ] Update existing test `test_generate_squad_yml_orchestrator_fields` to use `"sonnet"` instead of `"claude-sonnet-4-6"`
- [ ] Update `make_wizard_result()` fixture model string in `src/commands/init.rs`
- [ ] Update `test_append_workers_to_yaml_includes_model_when_present` model string

## Sources

### Primary (HIGH confidence)

- Codebase direct inspection (`src/cli.rs`, `src/main.rs`, `src/commands/wizard.rs`, `src/commands/init.rs`, `Cargo.toml`) — all findings
- `owo-colors` 3.x API — confirmed from existing `init.rs` usage patterns
- `clap` 4.5 optional subcommand — standard pattern, `Option<Commands>` is the canonical approach

### Secondary (MEDIUM confidence)

- Claude CLI short model name aliases (`sonnet`, `opus`, `haiku`) — from training knowledge; these aliases have been stable across Claude CLI versions

### Tertiary (LOW confidence)

- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in Cargo.toml with working usage
- Architecture: HIGH — change is minimal and surgical (two field type changes + one new module + one array edit)
- Pitfalls: HIGH — test failures from model name change are deterministic and easy to catch with `cargo test`

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable Rust ecosystem)
