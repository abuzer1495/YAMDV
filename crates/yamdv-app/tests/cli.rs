//! Windows integration tests for the CLI (prompt §4.8).
#![cfg(windows)]
#![allow(clippy::unwrap_used, missing_docs)]

use std::process::Command;

fn yamdv() -> Command {
    Command::new(env!("CARGO_BIN_EXE_yamdv"))
}

#[test]
fn r_sys_04_version_prints_and_exits_0() {
    let out = yamdv().arg("--version").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let s = String::from_utf8_lossy(&out.stdout);
    assert_eq!(s.trim(), format!("yamdv {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn r_sys_04_help_prints_and_exits_0() {
    let out = yamdv().arg("--help").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("--version"));
}

#[test]
fn r_sys_04_unknown_flag_exits_2() {
    let out = yamdv().arg("--definitely-not-a-flag").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--definitely-not-a-flag"));
}
