# Stack Research: Squad Station

**Project:** Squad Station — Rust CLI binary for routing messages between AI coding agents via tmux
**Researched:** 2026-03-06
**Research Mode:** Ecosystem

---

## Recommended Stack

### Core Framework

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| Rust | stable (1.86+) | Language | Project constraint; single binary, zero runtime, cross-compile |
| clap | 4.5.x (latest: 4.5.54) | CLI argument parsing | De-facto standard; derive macros eliminate boilerplate; subcommand architecture maps 1:1 to `squad-station send`, `signal`, `init`, `register`, `ui` |
| anyhow | 1.x | Application error handling | CLI apps show errors to users, not types — anyhow's context chain produces clear error messages without ceremony |
| thiserror | 2.x | Internal error types | Use in modules where callers need to distinguish errors; combine with anyhow at the boundary |
| tracing | 0.1.x | Structured logging | Better than `log` crate; async-ready (future-proof); RUST_LOG env var control; works with tracing-subscriber |
| tracing-subscriber | 0.3.x | Log output formatting | Pairs with tracing; supports env-filter for RUST_LOG |

**Rationale for clap:** The derive macro approach (`#[derive(Parser, Subcommand)]`) keeps command definitions co-located with handler structs. Squad Station has ~8 subcommands (`send`, `signal`, `init`, `register`, `list`, `view`, `ui`, `status`) — this complexity is exactly where clap shines over manual arg parsing.

**Rationale for anyhow:** Squad Station is an application, not a library. Error messages go to the terminal. `anyhow::Context` lets you add `"failed to send message to agent foo"` context at each layer without defining custom error types for every operation.

---

### Database

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| rusqlite | 0.38.0 | Embedded SQLite | Synchronous, SQLite-only — exactly right for a stateless CLI that runs, writes, and exits. No async overhead. |
| rusqlite (bundled feature) | 0.38.0 | SQLite static linking | `features = ["bundled"]` compiles SQLite 3.49.2 directly into the binary; zero system dependency; correct choice for single-binary distribution |
| rusqlite_migration | 2.4.0 | Schema migrations | Lightweight migration runner using SQLite's `user_version` PRAGMA instead of a migrations table; no CLI required; embed SQL strings or load from directory |

**Cargo.toml entry:**
```toml
rusqlite = { version = "0.38.0", features = ["bundled"] }
rusqlite_migration = "2.4.0"
```

**Rationale for rusqlite over sqlx:** Squad Station is stateless and synchronous — each command runs and exits. Async (sqlx) adds tokio runtime overhead with zero benefit. The consensus in 2025/2026 is: "SQLite-only project? Use rusqlite. Done." sqlx's compile-time query checking also requires a running database at build time, complicating CI.

**Rationale for bundled feature:** The binary must work on both macOS and Linux without any system SQLite dependency. `bundled` compiles SQLite 3.49.2 statically into the binary. Binary size cost is ~800KB — acceptable for a CLI tool.

**DB path pattern:** `~/.agentic-squad/<project>/station.db` — enforced by application code, not the database layer.

---

### TUI

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| ratatui | 0.30.x | TUI framework | Successor to archived tui-rs; 11.9M total downloads; 0.30 released January 2026 (biggest release yet); monorepo split into modular crates |
| crossterm | 0.28.x | Terminal backend | Default ratatui backend; cross-platform (Linux/macOS/Windows); event handling built in |

**Cargo.toml entry:**
```toml
ratatui = "0.30"
crossterm = "0.28"
```

**Rationale for ratatui:** tui-rs is archived — ratatui is its active fork with 3.2M recent downloads. Version 0.30 introduces `ratatui::run()` which simplifies terminal setup/teardown boilerplate. The widget set (Tables, Lists, Gauges, Sparklines, Tabs) covers everything the `squad-station ui` dashboard needs: agent status table, message log list, session indicators.

**Rationale for crossterm:** Default ratatui backend; works identically on macOS and Linux; handles terminal resize events and keyboard/mouse input. Termion (Linux-only) offers nothing over crossterm for this project.

**TUI scope:** The `squad-station ui` command is a read-only dashboard displaying agent states (idle/busy/dead), recent messages, and session names. It polls the SQLite DB and rerenders — no async required. Use a simple `loop { terminal.draw(...); handle_events(); }` pattern.

---

### CLI Framework (subcommand architecture)

The clap derive-based pattern for Squad Station's commands:

```rust
#[derive(Parser)]
#[command(name = "squad-station", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Send { agent: String, message: String },
    Signal { agent: String },
    Init { config: PathBuf },
    Register { name: String, session: String },
    List,
    Status { agent: Option<String> },
    View { agent: String },
    Ui,
}
```

Each subcommand is a function that takes `&Commands` variant and a DB connection. Stateless by design — no global state.

---

### tmux Integration

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| std::process::Command | (stdlib) | tmux invocation | **Recommended over tmux_interface crate** — see rationale below |

**Rationale for std::process::Command directly:**

`tmux_interface` (v0.3.2, last released March 2024) is explicitly marked "experimental/unstable" with API breakage warnings. It has 64 GitHub stars and 6 forks. The API surface doesn't justify the dependency risk for what Squad Station needs: two operations.

1. `tmux send-keys -t <session> "<message>" Enter` — inject prompt into agent pane
2. `tmux capture-pane -t <session> -p` — read pane content

Both are trivial with `std::process::Command`:

```rust
fn send_keys(session: &str, message: &str) -> anyhow::Result<()> {
    let status = std::process::Command::new("tmux")
        .args(["send-keys", "-t", session, message, "Enter"])
        .status()?;
    anyhow::ensure!(status.success(), "tmux send-keys failed");
    Ok(())
}

fn capture_pane(session: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new("tmux")
        .args(["capture-pane", "-t", session, "-p"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
```

This is 10 lines of stdlib code versus a pre-1.0 third-party crate with unstable API. No dependency needed.

**tmux presence detection:** Check `std::env::var("TMUX").is_ok()` or run `tmux ls` and check exit code. Surface a clear error if tmux is not running.

---

### Config File Parsing (squad.yml)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| serde | 1.0.x | Serialization framework | Standard; derive macros for `Deserialize` |
| serde-saphyr | latest | YAML parsing | serde_yaml is **deprecated** (archived March 2024); serde-saphyr is the actively maintained replacement — parses YAML directly to Rust types without intermediate syntax tree |

**Cargo.toml entry:**
```toml
serde = { version = "1.0", features = ["derive"] }
serde-saphyr = "0.1"  # verify latest on crates.io before use
```

**IMPORTANT — serde_yaml is dead:** Do not use `serde_yaml`. It was officially deprecated and archived in March 2024. The author explicitly stated no further versions will be published. The 0.9.34+deprecated version on crates.io is a tombstone. Alternatives as of 2026: `serde-saphyr` (pure Rust, actively developed), `serde_yml` (libyaml-based fork), `serde-yaml-bw` (community fork).

**Recommendation:** `serde-saphyr` for new projects — pure Rust, no C dependency (easier to audit, easier to cross-compile), actively developed.

---

### Distribution (npm wrapper)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| GitHub Actions | — | CI/CD and cross-compilation | Native macOS runners for Darwin targets; cross-rs for Linux targets |
| cross-rs | — | Linux cross-compilation | Docker-based; handles musl for static Linux binary; simpler than manual toolchain setup |
| npm optionalDependencies | — | Platform binary selection | Each platform gets its own npm package; npm installs only the matching one |
| Node.js wrapper script | — | Binary executor | Thin JS shim resolves correct binary from node_modules and execs it |

**Pattern (proven by esbuild, Biome, Turbopack, Bun):**

```
squad-station/                  # Root npm package (no binary)
  package.json                  # optionalDependencies pointing to platform packages
  bin/run.js                    # Node.js shim that finds + execs correct binary

squad-station-darwin-arm64/     # Binary package for Apple Silicon
  bin/squad-station             # Compiled binary for aarch64-apple-darwin

squad-station-darwin-x64/       # Binary package for Intel macOS
  bin/squad-station             # Compiled binary for x86_64-apple-darwin

squad-station-linux-x64/        # Binary package for Linux x86
  bin/squad-station             # Compiled binary for x86_64-unknown-linux-musl

squad-station-linux-arm64/      # Binary package for Linux ARM
  bin/squad-station             # Compiled binary for aarch64-unknown-linux-musl
```

**Node.js shim pattern:**
```javascript
#!/usr/bin/env node
const { execFileSync } = require('child_process');
const os = require('os');
const path = require('path');

const platform = os.platform();  // darwin, linux
const arch = os.arch();           // arm64, x64

const pkg = `squad-station-${platform}-${arch}`;
const binary = path.join(require.resolve(`${pkg}/package.json`), '..', 'bin', 'squad-station');

execFileSync(binary, process.argv.slice(2), { stdio: 'inherit' });
```

**GitHub Actions targets:**

| Target | Runner | Tool |
|--------|--------|------|
| aarch64-apple-darwin | macos-latest | cargo build (native) |
| x86_64-apple-darwin | macos-13 (Intel) | cargo build (native) |
| x86_64-unknown-linux-musl | ubuntu-latest | cross-rs |
| aarch64-unknown-linux-musl | ubuntu-latest | cross-rs |

**Rationale:** `optionalDependencies` with `os`/`cpu` fields in each platform package.json is the pattern npm natively supports. npm installs only the matching optional dependency. No postinstall script needed (avoids pnpm/yarn compatibility issues). This is what esbuild, Biome, and Bun use.

**Rationale against NAPI-RS:** NAPI-RS is for Node.js native addons (`.node` files loaded into the Node.js runtime). Squad Station is a standalone CLI binary — NAPI-RS is the wrong tool. The optionalDependencies pattern is the right pattern for binary distribution.

---

### Testing

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| Built-in `#[test]` | — | Unit tests | Pure Rust unit tests for routing logic, DB operations, config parsing |
| assert_cmd | 2.x | Integration tests | Tests CLI commands end-to-end as a subprocess; checks exit codes, stdout/stderr |
| tempfile | 3.x | Test DB isolation | Creates temporary directories/files for per-test SQLite databases |
| predicates | 3.x | Output assertions | Used with assert_cmd for rich stdout/stderr matching |

**Cargo.toml entry:**
```toml
[dev-dependencies]
assert_cmd = "2"
tempfile = "3"
predicates = "3"
```

**Testing strategy:** Unit tests for DB schema/queries and message routing logic. Integration tests with `assert_cmd` for CLI command invocation. tmux operations are the only component that requires a real tmux session — mock or skip in CI, test manually or with a tmux-present CI runner.

---

## What NOT to Use

| Technology | Reason to Avoid |
|------------|----------------|
| **tokio / async** | Squad Station is stateless and synchronous. Each command runs to completion. Async runtime adds 3–5MB binary overhead and tokio complexity for zero benefit. rusqlite is sync, tmux calls are sync, file I/O is sync. |
| **sqlx** | Requires async runtime; compile-time query checking needs a live DB at build time (complicates CI); overkill for SQLite-only single-binary CLI. rusqlite is the right tool. |
| **Diesel** | ORM learning curve; schema macro DSL; migration management overhead. Unnecessary for a simple 3-4 table schema with hand-written SQL. |
| **tui-rs** | Archived. ratatui is the maintained fork. Never start new projects on tui-rs. |
| **tmux_interface crate** | Pre-1.0, explicitly "experimental/unstable", 64 GitHub stars, last release March 2024. Wraps CLI calls that are 3 lines of stdlib code. Not worth the dependency. |
| **serde_yaml** | Officially deprecated and archived March 2024. Do not use. |
| **serde (postcard/bincode for config)** | squad.yml must be human-editable. Binary formats are wrong for user config files. |
| **NAPI-RS** | For Node.js native addons (`.node` files loaded into V8). Squad Station is a standalone CLI binary — wrong tool entirely. |
| **daemon/server architecture** | Project constraint: stateless CLI. No background process, no socket server. Each invocation reads DB, acts, writes DB, exits. |
| **clap builder pattern** | The older `App::new()` builder API is verbose and type-unsafe compared to derive macros. Always use `#[derive(Parser)]` in new 2025+ projects. |
| **env_logger** | Works fine but tracing is strictly better — structured fields, async compatibility, subscriber composability. No reason to use env_logger in a new project. |

---

## Confidence Levels

| Area | Confidence | Rationale |
|------|------------|-----------|
| clap 4.5.x | HIGH | Verified via crates.io (4.5.54 is latest as of Jan 2026); de-facto standard; multiple 2025-2026 tutorials confirm |
| rusqlite 0.38.0 + bundled | HIGH | Verified via crates.io and docs.rs; bundled SQLite 3.49.2; strong ecosystem consensus "use rusqlite for SQLite CLI tools" |
| rusqlite_migration 2.4.0 | HIGH | Verified via crates.io and docs.rs; actively maintained; well-documented |
| ratatui 0.30.x | HIGH | Official release notes verified via ratatui.rs; MSRV 1.86; released Jan 2026 |
| crossterm as ratatui backend | HIGH | Default ratatui backend per official docs; explicitly recommended for Linux/macOS/Windows |
| std::process::Command for tmux | HIGH | stdlib; zero risk; project's tmux surface is two commands; explicitly recommended over tmux_interface |
| serde-saphyr for YAML | MEDIUM | serde_yaml deprecation is confirmed HIGH; serde-saphyr is actively developed but newer/smaller community; verify crates.io version before use |
| npm optionalDependencies pattern | HIGH | Used by esbuild, Biome, Bun; pattern validated by Orhun's blog (ratatui maintainer) and Sentry Engineering |
| GitHub Actions + cross-rs | HIGH | Standard pattern; actions-rust-cross action is widely used; Darwin native runner strategy is established |
| anyhow + thiserror | HIGH | Industry standard for Rust CLI error handling; confirmed by multiple 2025-2026 sources |
| tracing + tracing-subscriber | HIGH | De-facto standard for structured logging in Rust since 2023; tokio ecosystem endorsement |

---

## Sources

- [clap 4.5.54 on crates.io](https://crates.io/crates/clap) — version verified
- [rusqlite 0.38.0 on crates.io](https://crates.io/crates/rusqlite/) — bundled SQLite 3.49.2 confirmed
- [rusqlite_migration 2.4.0 on docs.rs](https://docs.rs/crate/rusqlite_migration/latest) — version verified
- [Rust ORMs in 2026: Diesel vs SQLx vs SeaORM vs Rusqlite](https://aarambhdevhub.medium.com/rust-orms-in-2026-diesel-vs-sqlx-vs-seaorm-vs-rusqlite-which-one-should-you-actually-use-706d0fe912f3) — ecosystem consensus
- [ratatui v0.30.0 highlights](https://ratatui.rs/highlights/v030/) — official release notes, Jan 2026
- [ratatui Backends docs](https://ratatui.rs/concepts/backends/) — crossterm as default confirmed
- [Packaging Rust Applications for the NPM Registry](https://blog.orhun.dev/packaging-rust-for-npm/) — npm distribution pattern (by ratatui maintainer)
- [Publishing binaries on npm — Sentry Engineering](https://sentry.engineering/blog/publishing-binaries-on-npm) — optionalDependencies pattern
- [serde_yaml deprecation](https://docs.rs/crate/serde_yaml/latest) — 0.9.34+deprecated tombstone
- [serde-yaml alternatives forum thread](https://users.rust-lang.org/t/serde-yaml-deprecation-alternatives/108868) — community consensus on serde-saphyr
- [tmux-interface-rs GitHub](https://github.com/AntonGepting/tmux-interface-rs) — v0.3.2, last commit March 2025, experimental status confirmed
- [actions-rust-cross](https://github.com/houseabsolute/actions-rust-cross) — CI cross-compilation
- [Rust CLI error handling 2025](https://dev.to/leapcell/rust-error-handling-compared-anyhow-vs-thiserror-vs-snafu-2003) — anyhow/thiserror pattern
- [Building Rust CLI tools with clap 2026](https://oneuptime.com/blog/post/2026-02-03-rust-clap-cli-applications/view) — clap derive pattern
