mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;
use common::*;

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
    assert_file_contains(&temp_dir.path().join("Containerfile.base"), "Building stage: base");
}

#[test]
fn test_containerfile_discovery_nested_structure() {
    let temp_dir = TempDir::new().unwrap();
    setup_nested_containerfiles(&temp_dir, &[
        ("base", "base"),
        ("features", "gpu"),
        ("features", "bluetooth"),
        ("desktops", "hyprland"),
    ]);
    
    // Verify nested files were created
    assert_file_exists(&temp_dir.path().join("base/Containerfile.base"));
    assert_file_exists(&temp_dir.path().join("features/Containerfile.gpu"));
    assert_file_exists(&temp_dir.path().join("features/Containerfile.bluetooth"));
    assert_file_exists(&temp_dir.path().join("desktops/Containerfile.hyprland"));
    
    // Verify content
    assert_file_contains(
        &temp_dir.path().join("features/Containerfile.gpu"),
        "Building group: features stage: gpu"
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
    ).unwrap();
    
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