// Re-export internal modules for integration tests.
// This is the standard Rust approach: expose the library surface so `tests/` can import it.
pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod providers;
pub mod tmux;
