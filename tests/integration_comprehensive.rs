use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to create a test Git repository
fn create_test_repo() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("git init");

    // Configure git
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .expect("git config name");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .expect("git config email");

    // Create initial commit
    fs::write(repo_path.join("README.md"), "# Test Repository\n").expect("write file");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .expect("git add");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .expect("git commit");

    // Add more commits for testing
    for i in 1..=5 {
        fs::write(
            repo_path.join(format!("file{}.txt", i)),
            format!("Content {}\n", i),
        )
        .expect("write file");
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .expect("git add");
        Command::new("git")
            .args(["commit", "-m", &format!("Add file{}.txt", i)])
            .current_dir(&repo_path)
            .output()
            .expect("git commit");
    }

    (temp_dir, repo_path)
}

#[test]
fn test_basic_repository_status() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args(["--repo", repo_path.to_str().unwrap()]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Repository Information"))
        .stdout(predicate::str::contains("Current branch"))
        .stdout(predicate::str::contains("HEAD commit"));
}

#[test]
fn test_branches_listing() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args(["--repo", repo_path.to_str().unwrap(), "branches"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Branches"))
        .stdout(predicate::str::contains("main").or(predicate::str::contains("master")));
}

#[test]
fn test_log_output() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args(["--repo", repo_path.to_str().unwrap(), "log", "--count", "3"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Recent Commits"))
        .stdout(predicate::str::contains("Commit:"))
        .stdout(predicate::str::contains("Author:"));
}

#[test]
fn test_stats_repo_activity_json_validation() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "repo",
        "--since",
        "30d",
        "--bucket",
        "day",
        "--format",
        "json",
    ]);

    let output = cmd.output().expect("command runs");
    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");

    // Validate JSON structure matches our schema
    assert!(json.get("context").is_some(), "missing context");
    assert!(json.get("metric").is_some(), "missing metric");
    assert!(json.get("bucket").is_some(), "missing bucket");
    assert!(json.get("series").is_some(), "missing series");

    assert_eq!(json["metric"], "activity");
    assert_eq!(json["bucket"], "Day");
}

#[test]
fn test_stats_authors_comprehensive() {
    let (_temp_dir, repo_path) = create_test_repo();

    // Test table format
    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "authors",
        "--top",
        "5",
        "--metric",
        "commits",
        "--format",
        "table",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test User"));

    // Test JSON format
    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "authors",
        "--format",
        "json",
    ]);

    let output = cmd.output().expect("command runs");
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");

    assert!(json.get("authors").is_some());
    assert!(json["authors"].is_array());
}

#[test]
fn test_stats_calendar_output() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "calendar",
        "--format",
        "json",
    ]);

    let output = cmd.output().expect("command runs");
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");

    assert_eq!(json["kind"], "weekday_hour");
    assert!(json["matrix"].is_array());

    // Validate matrix dimensions (7 weekdays x 24 hours)
    let matrix = &json["matrix"];
    assert_eq!(matrix.as_array().unwrap().len(), 7);

    for day in matrix.as_array().unwrap() {
        assert_eq!(day.as_array().unwrap().len(), 24);
    }
}

#[test]
fn test_stats_hotspots_analysis() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "hotspots",
        "--top",
        "10",
        "--format",
        "table",
    ]);

    cmd.assert().success();

    // Test JSON output
    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "hotspots",
        "--format",
        "json",
    ]);

    let output = cmd.output().expect("command runs");
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");

    assert!(json.get("hotspots").is_some());
    assert!(json["hotspots"].is_array());
}

#[test]
fn test_stats_branches_analysis() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "branches",
        "--format",
        "table",
    ]);

    cmd.assert().success();
}

#[test]
fn test_stats_coupling_analysis() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "coupling",
        "--top",
        "5",
        "--format",
        "table",
    ]);

    cmd.assert().success();
}

#[test]
fn test_stats_ownership_analysis() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "ownership",
        "--top",
        "10",
        "--format",
        "table",
    ]);

    cmd.assert().success();
}

#[test]
fn test_stats_stability_analysis() {
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "stability",
        "--top",
        "10",
        "--format",
        "table",
    ]);

    cmd.assert().success();
}

#[test]
fn test_stats_releases_analysis() {
    let (_temp_dir, repo_path) = create_test_repo();

    // Add a tag for testing
    Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(&repo_path)
        .output()
        .expect("git tag");

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "stats",
        "releases",
        "--format",
        "table",
    ]);

    cmd.assert().success();
}

#[test]
fn test_interactive_mode_help() {
    // We can't fully test interactive mode, but we can test that it starts
    let (_temp_dir, repo_path) = create_test_repo();

    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args(["--repo", repo_path.to_str().unwrap(), "--help"]);

    cmd.assert().success().stdout(predicate::str::contains(
        "CLI tool for inspecting Git repos",
    ));
}

#[test]
fn test_format_options_validation() {
    let (_temp_dir, repo_path) = create_test_repo();

    // Test all valid formats
    for format in ["json", "table", "chart"] {
        let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
        cmd.args([
            "--repo",
            repo_path.to_str().unwrap(),
            "stats",
            "repo",
            "--format",
            format,
        ]);

        cmd.assert().success();
    }
}

#[test]
fn test_time_window_parsing() {
    let (_temp_dir, repo_path) = create_test_repo();

    // Test various time window formats
    let time_windows = ["30d", "12w", "6m", "1y"];

    for window in time_windows {
        let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
        cmd.args([
            "--repo",
            repo_path.to_str().unwrap(),
            "stats",
            "repo",
            "--since",
            window,
            "--format",
            "json",
        ]);

        cmd.assert().success();
    }
}

#[test]
fn test_bucket_size_options() {
    let (_temp_dir, repo_path) = create_test_repo();

    // Test all bucket sizes
    for bucket in ["day", "week", "month"] {
        let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
        cmd.args([
            "--repo",
            repo_path.to_str().unwrap(),
            "stats",
            "repo",
            "--bucket",
            bucket,
            "--format",
            "json",
        ]);

        cmd.assert().success();
    }
}

#[test]
fn test_error_handling_invalid_repo() {
    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args(["--repo", "/nonexistent/path"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to open repository"));
}

#[test]
fn test_performance_flags() {
    let (_temp_dir, repo_path) = create_test_repo();

    // Test debug flag
    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "--debug",
        "stats",
        "repo",
    ]);

    cmd.assert().success();

    // Test no-cache flag
    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "--no-cache",
        "stats",
        "repo",
    ]);

    cmd.assert().success();

    // Test max-threads flag
    let mut cmd = Command::cargo_bin("crabgit").expect("bin exists");
    cmd.args([
        "--repo",
        repo_path.to_str().unwrap(),
        "--max-threads",
        "2",
        "stats",
        "repo",
    ]);

    cmd.assert().success();
}
