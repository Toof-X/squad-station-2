# Stack Research: v2.0 Workflow Watchdog

**Domain:** Rust CLI — background polling watchdog, stall detection, Telegram alerting, tmux pane injection
**Researched:** 2026-03-24
**Confidence:** HIGH

> **Scope:** This document covers ONLY what is NEW or CHANGED for v2.0 Workflow Watchdog.
> The existing validated stack (ratatui 0.30, crossterm 0.29, tui-big-text 0.8, clap 4.5,
> tokio 1.37, sqlx 0.8, serde/serde_json 1.0, serde-saphyr, owo-colors 3, uuid 1.8, chrono 0.4,
> anyhow 1.0, libc 0.2, axum 0.7 [browser feature], rust-embed 8, futures 0.3) is NOT
> re-researched here. All findings below assume this baseline.

---

## Executive Summary

Two new crates are needed for v2.0: `reqwest` (HTTP client for Telegram Bot API) and
`tokio-util` (CancellationToken for graceful shutdown of the long-lived watchdog loop).
Both integrate directly into the existing tokio async runtime — no new runtime, no daemon
framework, no new DB dependencies.

The Telegram integration is intentionally thin: a direct POST to
`https://api.telegram.org/bot{token}/sendMessage` with a JSON body. No Telegram framework
(teloxide, frankenstein) is needed or wanted — squad-station sends alerts, it does not receive
messages or manage a bot lifecycle.

The watchdog polling loop follows the exact pattern established in `browser.rs`: a
`tokio::time::interval` tick driving DB reads, with `tokio::select!` for shutdown. The only
architectural addition is `tokio_util::sync::CancellationToken` to allow clean shutdown
from Ctrl+C without duplicating the signal handler.

The `tmux.rs` pane injection path already exists (`send_keys`). The watchdog reuses it
directly — the only new logic is choosing which pane to inject (orchestrator agent name
from DB) and constructing the alert message string.

---

## Recommended Stack

### New Crates to Add

| Crate | Version | Purpose | Why |
|-------|---------|---------|-----|
| `reqwest` | 0.12.x | Async HTTP client for Telegram Bot API POST | 0.12.x uses `http 1.0` + `hyper 1` — same underlying crates as `axum 0.7`. No duplicate `http` version in the binary. 0.13 switched TLS defaults to aws-lc (heavier), offers no benefit for a single outbound endpoint. |
| `tokio-util` | 0.7.x | `CancellationToken` for graceful watchdog shutdown | Ships with tokio-rs ecosystem; already likely a transitive dep via sqlx/axum. `CancellationToken` is the idiomatic cancellation primitive — cleaner than `tokio::sync::watch` channels for this use case. |

### Existing Crates Serving New Features

| Crate | Current Version | New Role in v2.0 |
|-------|----------------|-----------------|
| `tokio` | 1.37 | `tokio::time::interval` for polling loop; `tokio::select!` for shutdown; `tokio::signal` for Ctrl+C — all already used in `browser.rs` |
| `serde_json` | 1.0 | Build Telegram request body via `serde_json::json!` — same pattern used throughout codebase |
| `sqlx` | 0.8 | Read-only DB queries for stall detection (agent statuses + pending message counts) |
| `chrono` | 0.4 | `Utc::now()` for stall duration calculation — already imported |
| `anyhow` | 1.0 | Error propagation in `watchdog.rs` — same `anyhow::Result<()>` pattern as every other command |

---

## Feature-by-Feature Stack Analysis

### Feature 1: Long-Lived Background Polling Loop

**Pattern source:** `browser.rs` already implements this correctly. The watchdog copies it exactly.

```rust
// src/commands/watchdog.rs
pub async fn run(db_path: PathBuf, poll_interval_secs: u64) -> anyhow::Result<()> {
    let pool = db::connect_readonly(&db_path).await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_clone = cancel.clone();

    // Spawn signal handler that cancels the token
    tokio::spawn(async move {
        shutdown_signal().await;  // reuse browser.rs shutdown_signal() or inline
        cancel_clone.cancel();
    });

    let mut interval = tokio::time::interval(
        std::time::Duration::from_secs(poll_interval_secs)
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                break;
            }
            _ = interval.tick() => {
                check_for_stall(&pool).await?;
            }
        }
    }

    Ok(())
}
```

**Why this works with existing stack:**
- `tokio::time::interval` is part of `tokio` (already at 1.37 with `full` features)
- `tokio::select!` is a built-in macro
- `db::connect_readonly` is the same read-only pool pattern from `browser.rs` — prevents WAL contention

**Why NOT to add a scheduler crate (tokio-cron-scheduler, clokwerk):** The watchdog runs
continuously at a fixed interval (e.g., every 30 seconds). A scheduler crate adds hundreds
of KB and cron expression parsing for a single fixed-interval loop. `tokio::time::interval`
is exactly the right primitive.

---

### Feature 2: Stall Detection Logic

**No new crates needed.** Detection is pure DB query + conditional logic.

Stall condition: `pending_count > 0 OR processing_count > 0` AND `busy_agent_count == 0`.

```sql
-- One query — both conditions in a single round trip
SELECT
    COUNT(CASE WHEN m.status IN ('pending', 'processing') THEN 1 END) AS stuck_messages,
    COUNT(CASE WHEN a.status = 'busy' THEN 1 END) AS busy_agents
FROM messages m
CROSS JOIN agents a
WHERE a.role != 'orchestrator'
```

If `stuck_messages > 0 AND busy_agents == 0`: stall detected.

Add a `stall_since: Option<DateTime<Utc>>` in watchdog state (in-memory, not DB) to avoid
re-alerting every poll tick. Alert fires once when stall is first detected; a cooldown of
N minutes (configurable via CLI flag) prevents alert spam.

**Why NOT to add a `routing_hints` column or new schema:** Stall detection reads only
existing `messages.status` and `agents.status` columns — no schema migration needed.

---

### Feature 3: Telegram Alerting

**New crate: `reqwest` 0.12.x**

Use the Telegram Bot API directly. No bot framework.

```toml
# Cargo.toml addition — NOT feature-gated (watchdog always needs HTTP)
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

**Why `default-features = false`:**
- Eliminates native-tls, blocking, cookies, form, gzip, brotli, deflate — none needed
- `json` enables `.json(&payload)` on `RequestBuilder`
- `rustls-tls` provides TLS without system OpenSSL dependency — consistent with existing
  `sqlx` which uses `runtime-tokio-rustls`

**Why version 0.12.x, not 0.13.x:**
- `axum 0.7` depends on `http 1.0` and `hyper 1`
- `reqwest 0.12` also depends on `http 1.0` and `hyper 1` — **same version, no duplication**
- `reqwest 0.13` switched TLS defaults to `aws-lc` (heavier crypto, FIPS-focused) and changed
  feature flags (`query`/`form` disabled by default). No benefit for a single outbound POST.
- Binary size impact: reqwest 0.12 + rustls ~= 300KB added. Acceptable.

**Telegram send pattern:**

```rust
async fn send_telegram_alert(
    token: &str,
    chat_id: &str,
    message: &str,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let payload = serde_json::json!({
        "chat_id": chat_id,
        "text": message,
        "parse_mode": "HTML",
        "disable_notification": false,
    });
    let resp = client.post(&url).json(&payload).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Telegram API error: {}", resp.status());
    }
    Ok(())
}
```

**Configuration via CLI flags and env vars** (no new config file format needed):
- `--telegram-token <TOKEN>` or `SQUAD_TELEGRAM_TOKEN` env var
- `--telegram-chat-id <CHAT_ID>` or `SQUAD_TELEGRAM_CHAT_ID` env var
- Absent config = Telegram alerting silently disabled; tmux injection still fires

**Why NOT teloxide or frankenstein:**
- Both are bot frameworks for receiving and dispatching incoming Telegram messages
- Squad-station is an alert sender, not a bot server — no polling for updates, no handlers,
  no middleware, no webhook setup
- `frankenstein` alone pulls in `ureq` or `reqwest` anyway; we'd be wrapping a wrapper
- Direct POST to `sendMessage` is 10 lines of code with `reqwest` + `serde_json` (both
  already in scope or being added)

---

### Feature 4: Orchestrator tmux Pane Injection

**No new crates needed.** `tmux.rs` already has `send_keys(session_name, text)`.

The watchdog needs to:
1. Query the DB for the orchestrator agent name (`SELECT name FROM agents WHERE role = 'orchestrator' LIMIT 1`)
2. Call `tmux::send_keys(orchestrator_name, alert_text)` — existing function

The only new code is constructing the alert text string. Standard `format!` macro.

**Why NOT to add a new tmux primitive:** The existing `send_keys` function in `tmux.rs` uses
`tmux send-keys -l` (literal mode, injection-safe) — exactly what alert injection needs.
No modification to `tmux.rs` required; the watchdog calls the public function directly.

---

## Cargo.toml Changes

```toml
# Add to [dependencies] section:
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
tokio-util = { version = "0.7", features = ["rt"] }
```

`tokio-util` needs the `rt` feature for `CancellationToken`. If already a transitive dep,
this pin is still explicit — good practice.

**Feature gate decision:** The watchdog command does NOT need a feature gate. Unlike the
`browser` feature (which embeds an entire React SPA), the watchdog adds ~300KB (reqwest)
and is a core CLI command. Feature gates add complexity without benefit here.

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `reqwest 0.12` (no default features, json + rustls-tls) | `reqwest 0.13` | 0.13 defaults to aws-lc TLS (heavier). No benefit for single-endpoint sender. |
| `reqwest 0.12` | `hyper 1` directly | hyper requires manual HTTP/1.1 request construction. reqwest is the correct abstraction for a single outbound API call. |
| `reqwest 0.12` | `ureq` (sync) | ureq is synchronous. Tokio async runtime is mandatory — mixing sync HTTP inside async tasks causes thread blocking. |
| `reqwest 0.12` | `curl` via `std::process::Command` | Command-based curl loses structured error handling and adds process-spawn overhead per alert. |
| Direct `sendMessage` POST | `teloxide` framework | Teloxide is for building bots that receive messages. squad-station only sends — no incoming message handling needed. Framework is overkill by 10x. |
| Direct `sendMessage` POST | `frankenstein` crate | Thin wrapper that adds a layer over reqwest we're already using. Net LOC difference is ~5 lines. No justification. |
| `tokio_util::sync::CancellationToken` | `tokio::sync::watch` channel | watch requires sender + receiver setup and explicit channel threading. CancellationToken is purpose-built for this pattern, cleaner API. |
| `tokio_util::sync::CancellationToken` | `tokio::sync::oneshot` | oneshot fires exactly once but doesn't compose well with cloning for multi-task cancel. CancellationToken supports N clones. |
| `tokio::time::interval` for polling | `tokio-cron-scheduler` | Fixed interval requires no cron expression. Scheduler crate adds parsing overhead for a single loop. |
| In-memory `stall_since` state in watchdog | New DB column for stall state | DB column requires migration and write-per-poll. Stall state is ephemeral — it resets when the watchdog process restarts, which is correct behavior. |

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `teloxide` | Bot framework for message receiving — squad-station sends only | Direct `reqwest` POST to `sendMessage` endpoint |
| `frankenstein` | Thin wrapper over reqwest with no net benefit | Direct `reqwest` + `serde_json::json!` (10 lines) |
| `tokio-cron-scheduler` | Cron complexity for a fixed-interval loop | `tokio::time::interval` (already in tokio) |
| `tokio-graceful-shutdown` | Full supervisor framework for a single loop | `CancellationToken` from `tokio-util` is sufficient |
| Any daemon framework (`daemonize`, `service`) | Stateless CLI constraint — watchdog is a long-lived foreground process, not a system daemon | Run in tmux window or background shell session; user manages process lifecycle |
| New DB tables or columns | Stall detection reads existing `messages.status` + `agents.status`; alert state is in-memory | No migration needed |
| `native-tls` feature in reqwest | Adds system OpenSSL dependency; breaks musl static binary cross-compilation | `rustls-tls` feature in reqwest |

---

## Integration Points

| New Code | Integrates With | Integration Notes |
|----------|----------------|-------------------|
| `src/commands/watchdog.rs` | `src/cli.rs` | New `Commands::Watchdog { ... }` variant; match arm in `main.rs` |
| `src/commands/watchdog.rs` | `src/db/` | Read-only pool via `db::connect_readonly()` — same pattern as `browser.rs` |
| `src/commands/watchdog.rs` | `src/tmux.rs` | Calls `send_keys(orchestrator_name, alert_text)` — existing public function, no changes |
| `send_telegram_alert()` | `reqwest::Client` | Created once at watchdog startup, reused across alert calls (`Client` is `Clone` + connection pool) |
| `CancellationToken` | `tokio::signal` | Signal handler calls `token.cancel()`; loop exits on `token.cancelled()` — same pattern as `browser.rs` `shutdown_signal()` |

---

## Version Compatibility

| Package | Version | Compatibility Note |
|---------|---------|-------------------|
| `reqwest 0.12` | 0.12.x | Uses `http 1.0` + `hyper 1` — same as `axum 0.7`. No duplicate `http` crate version in binary. `rustls-tls` feature uses same `rustls` version as `sqlx`'s `runtime-tokio-rustls`. |
| `tokio-util 0.7` | 0.7.x | Maintained alongside `tokio 1.x` — same version series. `tokio-util 0.7` is compatible with `tokio 1.37`. |
| `reqwest 0.12` + `axum 0.7` | both use `http 1.0` | No type mismatches when used in same binary — they share the `http` crate, just don't share types across handler boundaries (which watchdog doesn't do). |

---

## Sources

- `Cargo.toml` (local) — confirmed existing dependency set; identified axum 0.7 uses http 1.0
- `src/commands/browser.rs` (local) — confirmed `connect_readonly`, `tokio::time::interval`, `tokio::select!`, `tokio::signal` patterns to reuse
- `src/tmux.rs` (local) — confirmed `send_keys` function is public and injection-safe
- [reqwest 0.13.2 on docs.rs](https://docs.rs/crate/reqwest/latest) — current version 0.13.2 (2026-02-06); verified feature flags; confirmed 0.12 still maintained (HIGH confidence)
- [reqwest GitHub CHANGELOG](https://github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md) — confirmed 0.12 → 0.13 TLS backend change (rustls + aws-lc default in 0.13) (HIGH confidence)
- [reqwest + axum 0.7 compatibility discussion](https://users.rust-lang.org/t/a-proxy-with-axum-0-7-and-reqwest-0-12-based-on-http-1/112489) — confirmed reqwest 0.12 shares http 1.0 with axum 0.7 (MEDIUM confidence)
- [CancellationToken in tokio-util 0.7.18](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html) — current version 0.7.18; CancellationToken confirmed present (HIGH confidence)
- [Telegram Bot API 9.5](https://core.telegram.org/bots/api) — current API version; `sendMessage` URL format and parameters confirmed (HIGH confidence)
- [Tokio graceful shutdown docs](https://tokio.rs/tokio/topics/shutdown) — CancellationToken pattern for multi-task shutdown confirmed idiomatic (HIGH confidence)

---

*Stack research for: squad-station v2.0 Workflow Watchdog — background polling, stall detection, Telegram alerting, tmux injection*
*Researched: 2026-03-24*
