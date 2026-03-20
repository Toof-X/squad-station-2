# Testing Patterns

**Analysis Date:** 2026-03-08

## Test Framework

**Runner:**
- Rust's built-in `cargo test` (no external runner)
- Async tests use `#[tokio::test]` macro (tokio runtime via `tokio = { version = "1.37", features = ["full"] }`)
- `tokio-test = "0.4"` present as dev-dependency

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_ne!` macros — no external assertion crate

**Run Commands:**
```bash
cargo test                          # Run all 58 unit + integration tests
cargo test test_name                # Run a single test by name (substring match)
cargo test --test test_commands     # Run a specific test file
cargo test --test test_db           # Run DB CRUD tests only
./tests/e2e_cli.sh                  # End-to-end CLI tests (requires release binary)
cargo build --release && ./tests/e2e_cli.sh  # Build then run E2E suite
```

## Test File Organization

**Location:**
- Unit tests: co-located in source via `#[cfg(test)] mod tests { ... }` blocks within the source file
- Integration tests: separate files under `tests/` directory

**Naming:**
- Integration test files: `tests/test_<area>.rs`
- Unit test functions: `test_<function>_<scenario>` — e.g., `test_format_status_with_duration_valid_timestamp`, `test_peek_priority_ordering`
- Integration test functions follow the same pattern

**Structure:**
```
tests/
├── helpers.rs              # Shared setup: setup_test_db()
├── test_cli.rs             # CLI parsing and Priority Display tests (sync)
├── test_commands.rs        # Config parsing and binary invocation tests (mixed)
├── test_db.rs              # DB CRUD layer tests (async, tokio)
├── test_integration.rs     # Full command integration tests (mixed)
├── test_lifecycle.rs       # Agent lifecycle + context + signal guard tests (mixed)
└── test_views.rs           # TUI App state + status/view command tests (mixed)

src/
├── commands/list.rs        # #[cfg(test)] mod tests { ... }
├── commands/status.rs      # #[cfg(test)] mod tests { ... }
├── tmux.rs                 # #[cfg(test)] mod tests { ... }
```

## Test Structure

**Suite Organization:**
```rust
// Section dividers group related tests in integration test files:
// ============================================================
// Agent CRUD tests
// ============================================================

#[tokio::test]
async fn test_insert_and_get_agent() {
    let pool = helpers::setup_test_db().await;
    // arrange
    agents::insert_agent(&pool, "frontend", "claude-code", "worker", "cmd").await.unwrap();
    // act
    let agent = agents::get_agent(&pool, "frontend").await.unwrap();
    // assert
    assert!(agent.is_some(), "agent should be present after insert");
    let agent = agent.unwrap();
    assert_eq!(agent.name, "frontend");
}
```

**Patterns:**
- Arrange-Act-Assert structure (implicit, not labeled)
- Each test gets its own isolated DB via `helpers::setup_test_db().await`
- Requirement codes referenced in test names and comments: `// MSG-03`, `// SESS-02`, `// HOOK-01`
- Async DB tests: `#[tokio::test]` + `helpers::setup_test_db()`
- Sync binary tests: `#[test]` + `std::process::Command::new(env!("CARGO_BIN_EXE_squad-station"))`
- `.expect("description")` used in test setup; `.unwrap()` used in assertions on known-good paths

## Mocking

**Framework:** None — no mock crate (no `mockall`, `mock_it`, etc.).

**Patterns:**
```rust
// Mock structs are constructed directly with literal field values:
fn mock_agent(name: &str, status: &str) -> squad_station::db::agents::Agent {
    squad_station::db::agents::Agent {
        id: "test-id".into(),
        name: name.into(),
        provider: "test".into(),
        role: "worker".into(),
        command: "echo".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        status: status.into(),
        status_updated_at: "2026-01-01T00:00:00Z".into(),
    }
}
```

**What to Mock:**
- Struct instances when testing TUI/UI state logic (e.g., `App` navigation in `tests/test_views.rs`)

**What NOT to Mock:**
- Database — use real SQLite via `setup_test_db()` (real file, WAL mode, migrations applied)
- tmux — commands that require tmux are either skipped or tested via binary invocation in an environment without tmux (errors handled gracefully)
- Filesystem — use `tempfile::TempDir` and `tempfile::NamedTempFile` for isolation

## Fixtures and Factories

**Test DB Setup:**
```rust
// tests/helpers.rs — call at the top of every async DB test
pub async fn setup_test_db() -> SqlitePool {
    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_owned();
    std::mem::forget(tmp); // keep file alive for test duration

    let opts = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("failed to create test pool");

    sqlx::migrate!("./src/db/migrations").run(&pool).expect("migrations");
    pool
}
```

**Squad YAML fixture** (used in binary/integration tests — duplicated across test files):
```rust
fn write_squad_yml(dir: &std::path::Path, db_file: &std::path::Path) {
    let yaml = format!(r#"project:
  name: test-squad
  db_path: "{db_path_str}"
orchestrator:
  name: test-orch
  provider: claude-code
  role: orchestrator
  command: "echo orch"
agents: []
"#);
    std::fs::write(dir.join("squad.yml"), yaml).expect("write squad.yml");
}
```

**File DB Setup** (used when binary tests need seeded data — also duplicated across files):
```rust
async fn setup_file_db(path: &std::path::Path) -> sqlx::SqlitePool {
    // real SQLite file, migrations applied, max_connections=1
}
```

**Location:** `tests/helpers.rs` for shared setup. `write_squad_yml` and `setup_file_db` are local helpers duplicated in `tests/test_integration.rs`, `tests/test_views.rs`, and `tests/test_lifecycle.rs`.

## Coverage

**Requirements:** 58 tests total (unit + integration). No enforced coverage percentage. No `cargo-tarpaulin` or similar configured.

**View Coverage:**
```bash
# No coverage tooling configured — run manually if needed:
cargo tarpaulin --out Html  # (not installed by default)
```

## Test Types

**Unit Tests (co-located, `#[cfg(test)]`):**
- Scope: pure functions that don't require DB or tmux
- Files: `src/tmux.rs`, `src/commands/list.rs`, `src/commands/status.rs`
- Examples: argument builder verification (`test_send_keys_args_have_literal_flag`), display formatting (`test_format_status_with_duration_hours`), padding logic (`test_pad_colored_adds_spaces`)

**DB Integration Tests (`tests/test_db.rs`):**
- Scope: full CRUD layer against real SQLite with migrations
- Async: all use `#[tokio::test]` + `helpers::setup_test_db()`
- Coverage: agent insert/get/list, message insert/update/list/peek, status transitions

**Command Integration Tests (`tests/test_integration.rs`, `tests/test_lifecycle.rs`, `tests/test_views.rs`):**
- Scope: binary invocation via `std::process::Command` with real DB files and `squad.yml`
- Mix of sync (`#[test]`) and async (`#[tokio::test]`) tests
- Pattern: seed DB via pool, close pool (`pool.close().await`), run binary, assert stdout/stderr/exit code
- Pool must be closed before binary invocation (single-writer constraint)

**TUI State Tests (`tests/test_views.rs`):**
- Scope: `App` struct state and navigation logic via mock agents
- All sync, no DB or tmux required
- Tests: `test_ui_navigation_next`, `test_ui_quit_key_q`, `test_ui_toggle_focus`, etc.

**CLI Parsing Tests (`tests/test_cli.rs`):**
- Scope: clap argument parsing validation via binary invocation
- All sync `#[test]`
- Tests: Priority display, valid/invalid flag values, default values shown in help

**E2E Shell Tests (`tests/e2e_cli.sh`):**
- Scope: full system test requiring release binary and live tmux sessions
- 16 sections (T1–T14), ~40 test cases
- Self-contained: creates tmpdir, writes `squad.yml`, starts/kills tmux sessions, cleans up on exit via `trap cleanup EXIT`
- Run: `./tests/e2e_cli.sh` (requires `cargo build --release` first)

## Common Patterns

**Async DB Test:**
```rust
#[tokio::test]
async fn test_update_status_idempotent() {
    // MSG-03: calling update_status twice returns 0 rows on the second call
    let pool = helpers::setup_test_db().await;
    agents::insert_agent(&pool, "agent-d", "claude-code", "worker", "cmd").await.unwrap();
    messages::insert_message(&pool, "agent-d", "task", "normal").await.unwrap();

    let first = messages::update_status(&pool, "agent-d").await.unwrap();
    assert_eq!(first, 1);

    let second = messages::update_status(&pool, "agent-d").await.unwrap();
    assert_eq!(second, 0, "second call must return 0 rows (idempotent, MSG-03)");
}
```

**Binary Invocation Test:**
```rust
#[tokio::test]
async fn test_list_json_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker-1", "claude", "worker", "echo").await.unwrap();
    db::messages::insert_message(&pool, "worker-1", "test task", "urgent").await.unwrap();
    pool.close().await;  // CRITICAL: close before binary runs (single-writer)

    write_squad_yml(tmp.path(), &db_path);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_squad-station"))
        .args(["list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");
    assert!(parsed.is_array());
    assert_eq!(parsed[0]["priority"], "urgent");
}
```

**Error Path Testing:**
```rust
// Verify exit code + error message content:
assert!(!output.status.success(), "send to nonexistent agent must fail");
let stderr = String::from_utf8_lossy(&output.stderr);
assert!(stderr.contains("Agent not found"), "got: {}", stderr);
```

**Timestamp-based Testing:**
```rust
// Small sleep for timestamp ordering (used sparingly):
tokio::time::sleep(std::time::Duration::from_millis(10)).await;
```

---

*Testing analysis: 2026-03-08*
