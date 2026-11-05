//! End-to-end checks of the shipped binary against a real catalogue file.
//!
//! These run the actual executable, so they cover argument parsing, exit codes
//! and the human/JSON output — not only the library.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

fn questline() -> Command {
    Command::cargo_bin("questline").expect("binary builds")
}

fn fixture(name: &str) -> String {
    format!("{}/../../fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn plan_prints_batches_for_a_mixed_catalogue() {
    questline()
        .args([
            "plan",
            "--catalogue",
            &fixture("games.json"),
            "--platform",
            "windows",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("batch 1/2"))
        .stdout(predicate::str::contains("batch 2/2"))
        .stdout(predicate::str::contains("Terraria"))
        .stdout(predicate::str::contains("cs2.exe"));
}

#[test]
fn plan_reports_rejected_entries_with_a_reason() {
    questline()
        .args([
            "plan",
            "--catalogue",
            &fixture("games.json"),
            "--platform",
            "windows",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Traversal Demo"))
        .stdout(predicate::str::contains("path traversal"))
        .stdout(predicate::str::contains("Reserved Device Name"))
        .stdout(predicate::str::contains("Bad Id"));
}

#[test]
fn plan_json_reports_the_same_numbers() {
    let output = questline()
        .args([
            "plan",
            "--catalogue",
            &fixture("games.json"),
            "--platform",
            "windows",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(parsed["platform"], "windows");
    assert_eq!(parsed["usable"], 4);
    assert_eq!(parsed["rejected"].as_array().expect("array").len(), 3);
    assert_eq!(parsed["without_executable"], 1);
    assert_eq!(parsed["batches"].as_array().expect("array").len(), 2);
    assert_eq!(parsed["batches"][0][0]["name"], "Terraria");
}

#[test]
fn a_target_limit_shrinks_the_plan() {
    let output = questline()
        .args([
            "plan",
            "--catalogue",
            &fixture("games.json"),
            "--platform",
            "windows",
            "--limit",
            "2",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(parsed["batches"].as_array().expect("array").len(), 1);
    assert_eq!(parsed["batches"][0].as_array().expect("batch").len(), 2);
}

#[test]
fn a_catalogue_with_nothing_usable_fails_loudly() {
    questline()
        .args([
            "plan",
            "--catalogue",
            &fixture("only-rejected.json"),
            "--platform",
            "windows",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("no usable games"));
}

#[test]
fn an_unreadable_catalogue_is_an_error_not_a_panic() {
    questline()
        .args(["plan", "--catalogue", "/definitely/not/here.json"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("cannot read"));
}

#[test]
fn an_out_of_range_batch_size_is_rejected() {
    questline()
        .args([
            "plan",
            "--catalogue",
            &fixture("games.json"),
            "--batch",
            "99",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("batch size"));
}
