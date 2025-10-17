use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn stats_repo_activity_json() {
    let mut cmd = Command::cargo_bin("gitcrab").expect("bin exists");
    cmd.args(["stats","repo","--since","30d","--bucket","week","--format","json"]);
    let out = cmd.output().expect("run");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["metric"], "activity");
}

#[test]
fn stats_repo_churn_json() {
    let mut cmd = Command::cargo_bin("gitcrab").expect("bin exists");
    cmd.args(["stats","repo","--since","90d","--metric","churn","--bucket","month","--format","json"]);
    let out = cmd.output().expect("run");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["metric"], "churn");
}

#[test]
fn stats_authors_table() {
    let mut cmd = Command::cargo_bin("gitcrab").expect("bin exists");
    cmd.args(["stats","authors","--since","90d","--top","3","--metric","commits","--format","table"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Top authors"));
}

#[test]
fn stats_calendar_json() {
    let mut cmd = Command::cargo_bin("gitcrab").expect("bin exists");
    cmd.args(["stats","calendar","--since","90d","--format","json"]);
    let out = cmd.output().expect("run");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["kind"], "weekday_hour");
}

#[test]
fn stats_hotspots_table() {
    let mut cmd = Command::cargo_bin("gitcrab").expect("bin exists");
    cmd.args(["stats","hotspots","--since","90d","--top","3","--format","table"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Hotspots"));
}
