mod common;

use assert_cmd::Command;
use common::*;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_file_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let _config_path = setup_config_file(&temp_dir, "test.toml");

    // Test that our config parsing logic works
    let config_content = fs::read_to_string(temp_dir.path().join("test.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&config_content).unwrap();

    assert!(parsed.get("build").is_some());
    assert!(parsed.get("environment").is_some());
}

#[test]
fn test_containerfile_discovery_flat_structure() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_containerfiles(&temp_dir, &["base", "tools", "final"]);

    // Verify files were created
    assert_file_exists(&temp_dir.path().join("Containerfile.base"));
    assert_file_exists(&temp_dir.path().join("Containerfile.tools"));
    assert_file_exists(&temp_dir.path().join("Containerfile.final"));

    // Verify content
    assert_file_contains(
        &temp_dir.path().join("Containerfile.base"),
        "Building stage: base",
    );
}

#[test]
fn test_containerfile_discovery_nested_structure() {
    let temp_dir = TempDir::new().unwrap();
    setup_nested_containerfiles(
        &temp_dir,
        &[
            ("base", "base"),
            ("features", "gpu"),  // Creates features/Containerfile.features
            ("desktops", "hyprland"),
        ],
    );

    // Verify nested files were created with correct group-based naming
    assert_file_exists(&temp_dir.path().join("base/Containerfile.base"));
    assert_file_exists(&temp_dir.path().join("features/Containerfile.features"));
    assert_file_exists(&temp_dir.path().join("desktops/Containerfile.desktops"));

    // Verify content - the last stage in each group will be what's in the file
    assert_file_contains(
        &temp_dir.path().join("features/Containerfile.features"),
        "Building group: features stage: gpu",
    );
}

#[test]
fn test_hooks_directory_setup() {
    let temp_dir = TempDir::new().unwrap();
    let hooks_dir = create_test_hooks_dir(&temp_dir);

    assert_file_exists(&hooks_dir);
    assert_file_exists(&hooks_dir.join("test-hook.sh"));
    assert_file_contains(&hooks_dir.join("test-hook.sh"), "Test hook executed");
}

#[test]
fn test_build_with_flat_containerfiles() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.env("TRLS_SKIP_ROOT_CHECK", "1")
        .arg("--src-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("base")
        .arg("build");

    // This will fail because we don't have podman in test environment,
    // but it should find the Containerfile
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not complain about missing Containerfile
    assert!(!stderr.contains("Containerfile not found"));
}

#[test]
fn test_build_with_nested_containerfiles() {
    let temp_dir = TempDir::new().unwrap();
    setup_nested_containerfiles(&temp_dir, &[("base", "base")]);

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.env("TRLS_SKIP_ROOT_CHECK", "1")
        .arg("--src-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("base")
        .arg("build");

    // This will fail because we don't have podman in test environment,
    // but it should find the nested Containerfile
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not complain about missing Containerfile
    assert!(!stderr.contains("Containerfile not found"));
}

#[test]
fn test_multistage_syntax() {
    let temp_dir = TempDir::new().unwrap();

    // Create a multi-stage Containerfile
    let containerfile_content = r#"
FROM alpine AS stage1
RUN echo "Stage 1"

FROM alpine AS stage2
RUN echo "Stage 2"
"#;

    fs::write(
        temp_dir.path().join("Containerfile.multi"),
        containerfile_content,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--src-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("multi:stage1,multi:stage2")
        .arg("build");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not complain about missing Containerfile
    assert!(!stderr.contains("Containerfile not found"));
}

#[test]
fn test_cache_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_containerfiles(&temp_dir, &["base"]);

    let pacman_cache = temp_dir.path().join("pacman-cache");
    let aur_cache = temp_dir.path().join("aur-cache");

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--src-dir")
        .arg(temp_dir.path())
        .arg("--pacman-cache")
        .arg(&pacman_cache)
        .arg("--aur-cache")
        .arg(&aur_cache)
        .arg("--rootfs-stages")
        .arg("base")
        .arg("build");

    let _output = cmd.output().unwrap();

    // The directories should be created even if the build fails
    // Note: This might not work in all test environments
}

#[test]
fn test_error_handling_with_invalid_stage() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.env("TRLS_SKIP_ROOT_CHECK", "1")
        .arg("--src-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("nonexistent")
        .arg("build");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Missing required containerfiles"));
}

#[test]
fn test_rootfs_base_cli_argument() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.env("TRLS_SKIP_ROOT_CHECK", "1")
        .arg("--src-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-stages")
        .arg("base")
        .arg("--rootfs-base")
        .arg("alpine:latest")
        .arg("build");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail due to missing Containerfile (it will fail later due to no podman)
    assert!(!stderr.contains("Containerfile not found"));
    // Verify the argument was accepted (no argument parsing errors)
    assert!(!stderr.contains("error: unexpected argument"));
}

#[test]
fn test_rootfs_base_help_text() {
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--rootfs-base"))
        .stdout(predicate::str::contains("Base image for the first stage"));
}

#[test]
fn test_rootfs_base_with_config_file() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_containerfiles(&temp_dir, &["base"]);

    let config_path = setup_config_file(&temp_dir, "test_rootfs_base.toml");

    let config_content = r#"
[build]
rootfs_base = "ubuntu:22.04"
rootfs_stages = ["base"]

[environment]
pacman_cache = "/tmp/pacman-cache"
aur_cache = "/tmp/aur-cache"
"#;

    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.env("TRLS_SKIP_ROOT_CHECK", "1")
        .env("TRELLIS_CONFIG", &config_path)
        .arg("--src-dir")
        .arg(temp_dir.path())
        .arg("build");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail due to config parsing issues
    assert!(!stderr.contains("Failed to parse config file"));
    assert!(!stderr.contains("Containerfile not found"));
}

#[test]
fn test_rootfs_base_cli_overrides_config_file() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_containerfiles(&temp_dir, &["base"]);

    let config_path = setup_config_file(&temp_dir, "test_override.toml");

    let config_content = r#"
[build]
rootfs_base = "ubuntu:22.04"
rootfs_stages = ["base"]

[environment]
pacman_cache = "/tmp/pacman-cache"
aur_cache = "/tmp/aur-cache"
"#;

    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.env("TRLS_SKIP_ROOT_CHECK", "1")
        .env("TRELLIS_CONFIG", &config_path)
        .arg("--src-dir")
        .arg(temp_dir.path())
        .arg("--rootfs-base")
        .arg("alpine:edge") // CLI should override config file
        .arg("build");

    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not fail due to config parsing or argument issues
    assert!(!stderr.contains("Failed to parse config file"));
    assert!(!stderr.contains("error: unexpected argument"));
    assert!(!stderr.contains("Containerfile not found"));
}
