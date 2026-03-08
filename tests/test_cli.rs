mod helpers;

use squad_station::cli::Priority;

// ============================================================
// Priority Display trait tests
// ============================================================

#[test]
fn test_priority_display_normal() {
    assert_eq!(Priority::Normal.to_string(), "normal");
}

#[test]
fn test_priority_display_high() {
    assert_eq!(Priority::High.to_string(), "high");
}

#[test]
fn test_priority_display_urgent() {
    assert_eq!(Priority::Urgent.to_string(), "urgent");
}

// ============================================================
// CLI argument parsing via binary — clap integration
// ============================================================

#[test]
fn test_cli_version_flag() {
    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success(), "--version must exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("squad-station"),
        "--version output must contain binary name, got: {}",
        stdout
    );
}

#[test]
fn test_cli_unknown_subcommand_fails() {
    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
        .arg("nonexistent-command")
        .output()
        .expect("failed to run binary");

    assert!(
        !output.status.success(),
        "unknown subcommand must exit non-zero"
    );
}

#[test]
fn test_cli_send_missing_args_fails() {
    // `send` requires agent and task positional args
    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
        .arg("send")
        .output()
        .expect("failed to run binary");

    assert!(
        !output.status.success(),
        "send without args must exit non-zero"
    );
}

#[test]
fn test_cli_send_priority_flag_accepts_valid_values() {
    // Verify clap accepts all priority values (will fail on missing squad.yml, not on parsing)
    let bin = env!("CARGO_BIN_EXE_squad-station");
    for priority in &["normal", "high", "urgent"] {
        let output = std::process::Command::new(bin)
            .args(["send", "agent", "task", "--priority", priority])
            .current_dir(std::env::temp_dir()) // no squad.yml — will fail after parsing
            .output()
            .expect("failed to run binary");

        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should fail due to missing squad.yml, NOT due to invalid priority parsing
        assert!(
            !stderr.contains("invalid value"),
            "priority '{}' must be accepted by clap, got stderr: {}",
            priority,
            stderr
        );
    }
}

#[test]
fn test_cli_send_priority_flag_rejects_invalid() {
    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
        .args(["send", "agent", "task", "--priority", "critical"])
        .output()
        .expect("failed to run binary");

    assert!(
        !output.status.success(),
        "invalid priority must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value"),
        "clap should report invalid value for priority, got: {}",
        stderr
    );
}

#[test]
fn test_cli_list_default_limit() {
    // list subcommand help should show default_value for limit
    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
        .args(["list", "--help"])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("20"),
        "list help must show default limit of 20, got: {}",
        stdout
    );
}

#[test]
fn test_cli_register_default_role_and_tool() {
    // register subcommand help should show defaults for role and tool
    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
        .args(["register", "--help"])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("worker"),
        "register help must show default role 'worker', got: {}",
        stdout
    );
    assert!(
        stdout.contains("unknown"),
        "register help must show default tool 'unknown', got: {}",
        stdout
    );
    assert!(
        stdout.contains("tool"),
        "register help must mention --tool flag, got: {}",
        stdout
    );
}
