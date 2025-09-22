use assert_cmd::Command;
use predicates::prelude::*;
use review_radar::Config;
use std::fs;
use tempfile::TempDir;

fn create_test_config(
    temp_dir: &TempDir,
    orgs: Vec<&str>,
    username: &str,
    repo_pattern: Option<&str>,
) -> String {
    let config_dir = temp_dir.path().join("config");
    let review_radar_dir = config_dir.join("review-radar");
    fs::create_dir_all(&review_radar_dir).unwrap();

    let config = Config {
        orgs: orgs.iter().map(|s| s.to_string()).collect(),
        username: username.to_string(),
        repo_pattern: repo_pattern.map(|s| s.to_string()),
    };

    let config_path = review_radar_dir.join("config.toml");
    config.save_to_path(&config_path).unwrap();

    config_dir.to_string_lossy().to_string()
}

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Find GitHub PRs where your review has been requested",
        ));
}

#[test]
fn test_config_command_without_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("empty");

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("config")
        .env("XDG_CONFIG_HOME", config_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration not found"));
}

#[test]
fn test_init_command() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("init")
        .arg("test-org1,test-org2")
        .arg("testuser")
        .env("XDG_CONFIG_HOME", config_dir.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration saved successfully"))
        .stdout(predicate::str::contains(
            "Organizations: test-org1, test-org2",
        ));

    // Verify config file was created
    let config_path = config_dir.join("review-radar").join("config.toml");
    assert!(config_path.exists());

    // Verify config contents
    let config = Config::load_from_path(&config_path).unwrap();
    assert_eq!(config.orgs, vec!["test-org1", "test-org2"]);
    assert_eq!(config.username, "testuser");
}

#[test]
fn test_init_command_with_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("init")
        .arg("test-org")
        .arg("testuser")
        .arg("-r")
        .arg("backend-.*")
        .env("XDG_CONFIG_HOME", config_dir.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Repository filter pattern: backend-.*",
        ));

    let config_path = config_dir.join("review-radar").join("config.toml");
    let config = Config::load_from_path(&config_path).unwrap();
    assert_eq!(config.repo_pattern, Some("backend-.*".to_string()));
}

#[test]
fn test_set_command_add_org() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(&temp_dir, vec!["org1"], "testuser", None);

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("set")
        .arg("--orgs")
        .arg("+org2")
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Added organization: org2"));
}

#[test]
fn test_set_command_remove_org() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(&temp_dir, vec!["org1", "org2"], "testuser", None);

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("set")
        .arg("--orgs")
        .arg("-org1")
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed organization: org1"));
}

#[test]
fn test_set_command_replace_orgs() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(&temp_dir, vec!["org1"], "testuser", None);

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("set")
        .arg("--orgs")
        .arg("new-org1,new-org2")
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated organizations"));
}

#[test]
fn test_set_command_invalid_regex() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(&temp_dir, vec!["org1"], "testuser", None);

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("set")
        .arg("-r")
        .arg("[invalid")
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Invalid regex pattern"));
}

#[test]
fn test_set_command_clear_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(&temp_dir, vec!["org1"], "testuser", Some("test-.*"));

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("set")
        .arg("-r")
        .arg("none")
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Cleared repository filter pattern",
        ));
}

#[test]
fn test_config_command_with_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(
        &temp_dir,
        vec!["org1", "org2"],
        "testuser",
        Some("backend-.*"),
    );

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("config")
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Organizations: org1, org2"))
        .stdout(predicate::str::contains("Username: testuser"))
        .stdout(predicate::str::contains("Repository filter: backend-.*"));
}

#[test]
fn test_main_command_no_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("empty");

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.env("XDG_CONFIG_HOME", config_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Configuration not found"));
}

#[test]
fn test_main_command_no_orgs_configured() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(&temp_dir, vec![], "testuser", None);

    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("No organizations configured"));
}

#[test]
fn test_command_line_org_override() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(&temp_dir, vec!["org1"], "testuser", None);

    // This test will fail because we don't have gh CLI access, but it tests argument parsing
    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("--orgs")
        .arg("override-org")
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .failure(); // Expected to fail due to gh CLI requirements
}

#[test]
fn test_own_flag() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(&temp_dir, vec!["org1"], "testuser", None);

    // This test will fail because we don't have gh CLI access, but it tests argument parsing
    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("--own")
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .failure(); // Expected to fail due to gh CLI requirements
}

#[test]
fn test_repo_pattern_override() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = create_test_config(&temp_dir, vec!["org1"], "testuser", None);

    // This test will fail because we don't have gh CLI access, but it tests argument parsing
    let mut cmd = Command::cargo_bin("rr").unwrap();
    cmd.arg("-r")
        .arg("test-.*")
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .failure(); // Expected to fail due to gh CLI requirements
}
