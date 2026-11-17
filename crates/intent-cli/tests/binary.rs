//! Runs the real binary once, to prove the wiring from `main` works.

use std::process::Command;

const V0: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../layouts/intent-v0.toml");

#[test]
fn check_passes_on_the_shipped_layout() {
    let output = Command::new(env!("CARGO_BIN_EXE_intent"))
        .args(["check", V0, "--strict"])
        .output()
        .expect("the intent binary runs");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 output");
    assert!(stdout.starts_with("intent-v0 0.2.0: ok"), "{stdout}");
}

#[test]
fn a_missing_layout_exits_with_code_two() {
    let output = Command::new(env!("CARGO_BIN_EXE_intent"))
        .args(["check", "no/such/layout.toml"])
        .output()
        .expect("the intent binary runs");
    assert_eq!(output.status.code(), Some(2));
}
