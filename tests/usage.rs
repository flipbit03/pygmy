//! Runs the real binary with an empty config dir (no API calls).

use std::process::Command;

fn pygmy(args: &[&str]) -> (String, String, bool) {
    let config = std::env::temp_dir().join(format!("pygmy-usage-test-{}", std::process::id()));
    std::fs::create_dir_all(&config).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pygmy"))
        .args(args)
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_CACHE_HOME", &config)
        .output()
        .unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.success(),
    )
}

#[test]
fn usage_with_empty_config_says_not_configured() {
    let (stdout, _, ok) = pygmy(&["usage"]);
    assert!(ok);
    assert!(stdout.contains("WHEN TO NOTIFY:"));
    assert!(stdout.contains("--file"));
    assert!(stdout.contains("BACKENDS: not configured"), "{stdout}");
    assert!(stdout.contains("pygmy init <backend>"));
}

#[test]
fn message_starting_with_dash_needs_double_dash() {
    // Documented in usage: clap rejects a leading "-" unless "--" comes first.
    let (_, stderr, ok) = pygmy(&["--topic", "t", "-3 tests failing"]);
    assert!(!ok);
    assert!(stderr.contains("unexpected argument"), "{stderr}");

    // With "--" the message is accepted (and fails later, on the missing config).
    let (_, stderr, ok) = pygmy(&["--topic", "t", "--", "-3 tests failing"]);
    assert!(!ok);
    assert!(stderr.contains("config"), "{stderr}");
}
