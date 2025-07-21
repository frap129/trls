//! Comprehensive tests for Trellis core functionality.
//!
//! Tests cover the core coordination logic between builder, cleaner, and runner components.

mod common;

use common::mocks::*;
use mockall::predicate;
use std::sync::Arc;
use tempfile::TempDir;
use trellis::{config::TrellisConfig, trellis::Trellis};

fn create_test_config(temp_dir: &TempDir) -> TrellisConfig {
    TrellisConfig {
        builder_stages: vec!["base".to_string()],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec!["base".to_string(), "final".to_string()],
        rootfs_base: "scratch".to_string(),
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
    }
}

#[test]
fn test_trellis_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());

    let trellis = Trellis::new(&config, executor);
    // Test passes if no panic occurs during creation
}

#[test]
fn test_build_builder_container_success() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let config = create_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_ok());
}

#[test]
fn test_build_rootfs_container_success() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "final"]);

    let config = create_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_rootfs_container();
    assert!(result.is_ok());
}

#[test]
fn test_run_rootfs_container() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let args = vec!["echo".to_string(), "hello".to_string()];
    let result = trellis.run_rootfs_container(&args);
    assert!(result.is_ok());
}

#[test]
fn test_clean_operation() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::multiple_images());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.clean();
    assert!(result.is_ok());
}

#[test]
fn test_update_operation() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "final"]);

    let config = create_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.update();
    assert!(result.is_ok());
}

#[test]
fn test_build_builder_with_empty_stages() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_test_config(&temp_dir);
    config.builder_stages = vec![];

    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No builder stages defined"));
}

#[test]
fn test_build_rootfs_with_empty_stages() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_test_config(&temp_dir);
    config.rootfs_stages = vec![];

    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_rootfs_container();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No rootfs stages defined"));
}

#[test]
fn test_build_with_missing_containerfiles() {
    let temp_dir = TempDir::new().unwrap();
    // Don't create containerfiles

    let config = create_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required containerfiles"));
}

#[test]
fn test_build_with_command_failures() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let config = create_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::build_failures());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_err());
}

#[test]
fn test_multi_stage_build_coordination() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "tools", "final"]);

    let mut config = create_test_config(&temp_dir);
    config.rootfs_stages = vec!["base".to_string(), "tools".to_string(), "final".to_string()];

    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_rootfs_container();
    assert!(result.is_ok());
}

#[test]
fn test_auto_clean_integration() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_test_config(&temp_dir);
    config.auto_clean = true;

    let executor = Arc::new(MockScenarios::multiple_images());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_ok());
}

#[test]
fn test_cache_directory_handling() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_test_config(&temp_dir);
    config.pacman_cache = Some(temp_dir.path().join("pacman"));
    config.aur_cache = Some(temp_dir.path().join("aur"));

    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_ok());
}

#[test]
fn test_custom_rootfs_base_configuration() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "final"]);

    let mut config = create_test_config(&temp_dir);
    config.rootfs_base = "fedora:39".to_string();

    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_rootfs_container();
    assert!(result.is_ok());
}

#[test]
fn test_extra_contexts_configuration() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_test_config(&temp_dir);
    config.extra_contexts = vec!["context1=/tmp".to_string(), "context2=/opt".to_string()];

    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_ok());
}

#[test]
fn test_extra_mounts_configuration() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_test_config(&temp_dir);
    config.extra_mounts = vec![
        "mount1=/var/cache".to_string().into(),
        "mount2=/var/log".to_string().into(),
    ];

    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_ok());
}

#[test]
fn test_podman_build_cache_configuration() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_test_config(&temp_dir);
    config.podman_build_cache = true;

    let executor = Arc::new(MockScenarios::all_success());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_ok());
}

#[test]
fn test_error_propagation_from_builder() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let config = create_test_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_build()
        .returning(|_| Err(anyhow::anyhow!("Build command failed")));

    let executor = Arc::new(mock_executor);
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Build command failed"));
}

#[test]
fn test_error_propagation_from_cleaner() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_test_config(&temp_dir);
    config.auto_clean = true;

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_build()
        .returning(|_| Ok(create_success_output("Build successful")));
    mock_executor
        .expect_podman_images()
        .returning(|_| Err(anyhow::anyhow!("Images command failed")));

    let executor = Arc::new(mock_executor);
    let trellis = Trellis::new(&config, executor);

    let result = trellis.build_builder_container();
    assert!(result.is_err());
    
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to list podman images"));
}

#[test]
fn test_error_propagation_from_runner() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_execute()
        .with(predicate::eq("podman"), predicate::function(|args: &[String]| {
            args.len() >= 2 && args[0] == "image" && args[1] == "exists"
        }))
        .returning(|_, _| Ok(create_success_output(""))); // Image exists check passes
    mock_executor
        .expect_podman_run()
        .returning(|_| Err(anyhow::anyhow!("Run command failed")));

    let executor = Arc::new(mock_executor);
    let trellis = Trellis::new(&config, executor);

    let args = vec!["echo".to_string(), "hello".to_string()];
    let result = trellis.run_rootfs_container(&args);
    assert!(result.is_err());
    
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to execute podman run command"));
}

#[test]
fn test_update_failure_in_build_phase() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "final"]);

    let config = create_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::build_failures());
    let trellis = Trellis::new(&config, executor);

    let result = trellis.update();
    assert!(result.is_err());
}

#[test]
fn test_update_failure_in_bootc_phase() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "final"]);

    let config = create_test_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_build()
        .returning(|_| Ok(create_success_output("Build successful")));
    mock_executor.expect_podman_images().returning(|_| {
        Ok(create_success_output(
            "REPOSITORY\tTAG\tIMAGE ID\tCREATED\tSIZE\n",
        ))
    });
    mock_executor
        .expect_bootc()
        .returning(|_| Err(anyhow::anyhow!("Bootc upgrade failed")));

    let executor = Arc::new(mock_executor);
    let trellis = Trellis::new(&config, executor);

    let result = trellis.update();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Bootc upgrade failed"));
}
