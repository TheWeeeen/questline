#![allow(clippy::expect_used, clippy::unwrap_used)]
use std::path::PathBuf;

use questline_core::{sanitize_executable_rel_path, PathError, Platform};

fn win(raw: &str) -> Result<PathBuf, PathError> {
    sanitize_executable_rel_path(raw, Platform::Windows)
}

fn unix(raw: &str) -> Result<PathBuf, PathError> {
    sanitize_executable_rel_path(raw, Platform::Unix)
}

#[test]
fn plain_name_gets_an_exe_suffix_on_windows() {
    assert_eq!(win("Terraria").expect("ok"), PathBuf::from("Terraria.exe"));
    assert_eq!(unix("Terraria").expect("ok"), PathBuf::from("Terraria"));
}

#[test]
fn the_leading_marker_discord_uses_is_stripped() {
    assert_eq!(
        win(">MassEffect.exe").expect("ok"),
        PathBuf::from("MassEffect.exe")
    );
    assert_eq!(win("  > cs2.exe  ").expect("ok"), PathBuf::from("cs2.exe"));
}

#[test]
fn nested_relative_paths_are_kept() {
    assert_eq!(
        win("bin/win64/game.exe").expect("ok"),
        PathBuf::from("bin").join("win64").join("game.exe")
    );
    assert_eq!(
        unix("game/bin/linux/game").expect("ok"),
        PathBuf::from("game/bin/linux/game")
    );
}

#[test]
fn traversal_is_rejected_not_rewritten() {
    for raw in ["../../evil.exe", "a/../../evil.exe", "..", "./game.exe"] {
        let err = win(raw).expect_err("must be rejected");
        assert!(
            matches!(err, PathError::Traversal(_)),
            "{raw} produced {err:?}"
        );
    }
}

#[test]
fn absolute_paths_are_rejected() {
    for raw in [
        "/usr/bin/game",
        "C:\\Windows\\System32\\cmd.exe",
        "c:game.exe",
    ] {
        let err = win(raw).expect_err("must be rejected");
        assert!(
            matches!(err, PathError::Absolute(_)),
            "{raw} produced {err:?}"
        );
    }
}

#[test]
fn windows_forbidden_characters_are_removed() {
    assert_eq!(win("ga:me?.exe").expect("ok"), PathBuf::from("game.exe"));
    assert_eq!(
        win("Game<1>|2.exe").expect("ok"),
        PathBuf::from("Game12.exe")
    );
}

#[test]
fn unix_keeps_characters_windows_would_hate() {
    assert_eq!(unix("ga:me.exe").expect("ok"), PathBuf::from("ga:me.exe"));
}

#[test]
fn trailing_dots_and_spaces_are_trimmed_on_windows() {
    assert_eq!(win("game.exe. . ").expect("ok"), PathBuf::from("game.exe"));
}

#[test]
fn reserved_windows_device_names_are_rejected() {
    for raw in ["CON", "con.exe", "NUL.exe", "com1.exe", "LPT9"] {
        let err = win(raw).expect_err("must be rejected");
        assert!(
            matches!(err, PathError::ReservedDeviceName(_)),
            "{raw} produced {err:?}"
        );
    }
    // The same name is legal on Unix.
    assert!(unix("con.exe").is_ok());
}

#[test]
fn names_that_sanitize_to_nothing_are_rejected() {
    for raw in ["", "   ", ">", "?.exe", "<>|"] {
        assert!(win(raw).is_err(), "{raw:?} must be rejected");
    }
}
