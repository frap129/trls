//! Comprehensive tests for ContainerRunner functionality.
//!
//! Tests cover container execution, bootc operations, and validation logic.

mod common;

use common::{mocks::*, TestVariation};
use std::sync::Arc;
use tempfile::TempDir;
use trellis::{
    config::TrellisConfig,
    trellis::runner::{ContainerCapability, ContainerRunner, PodmanRunCommandBuilder},
};

fn create_runner_config(temp_dir: &TempDir) -> TrellisConfig {
    TrellisConfig {
        builder_stages: vec!["base".to_string()],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec!["base".to_string()],
        rootfs_base: "scratch".to_string(),
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
        quiet: false,
    }
}

#[test]
fn test_container_runner_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());

    let _runner = ContainerRunner::new(&config, executor);
    // Test passes if no panic occurs during creation
}

fn test_run_container_success_impl(variation: TestVariation) {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_runner_config(&temp_dir);
    variation.apply_to_config(&mut config);

    let executor = if variation.quiet {
        let mut mock_executor = MockCommandExecutor::new();
        mock_executor
            .expect_execute()
            .returning(|_, _| Ok(create_success_output("Image exists")));
        mock_executor
            .expect_podman_run()
            .returning(|_| Ok(create_success_output("Container executed")));
        Arc::new(mock_executor)
    } else {
        Arc::new(MockScenarios::all_success())
    };

    let runner = ContainerRunner::new(&config, executor);
    let args = vec!["echo".to_string(), "hello".to_string()];
    let result = runner.run_container("test-rootfs", &args);
    assert!(result.is_ok());
}

#[test]
fn test_run_container_success_standard() {
    test_run_container_success_impl(TestVariation::standard());
}

#[test]
fn test_run_container_success_quiet() {
    test_run_container_success_impl(TestVariation::quiet());
}

#[test]
fn test_run_container_with_empty_args() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let runner = ContainerRunner::new(&config, executor);

    let args = vec![];
    let result = runner.run_container("test-rootfs", &args);
    assert!(result.is_ok());
}

#[test]
fn test_run_container_with_complex_args() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let runner = ContainerRunner::new(&config, executor);

    let args = vec![
        "/bin/bash".to_string(),
        "-c".to_string(),
        "echo 'hello world' && ls -la".to_string(),
    ];
    let result = runner.run_container("test-rootfs", &args);
    assert!(result.is_ok());
}

#[test]
fn test_run_container_image_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_execute()
        .returning(|_, _| Ok(create_failure_output("Image not found")));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let args = vec!["echo".to_string(), "hello".to_string()];
    let result = runner.run_container("nonexistent", &args);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Container image not found"));
}

#[test]
fn test_run_container_execution_failure() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    // Image exists check succeeds
    mock_executor
        .expect_execute()
        .with(
            mockall::predicate::eq("podman"),
            mockall::predicate::function(|args: &[String]| {
                args.len() >= 3 && args[0] == "image" && args[1] == "exists"
            }),
        )
        .returning(|_, _| Ok(create_success_output("Image exists")));

    // But run command fails
    mock_executor
        .expect_podman_run_streaming()
        .returning(|_| Ok(common::mocks::create_failure_status()));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let args = vec!["echo".to_string(), "hello".to_string()];
    let result = runner.run_container("test-rootfs", &args);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Podman run failed"));
}

fn test_run_bootc_upgrade_success_impl(variation: TestVariation) {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_runner_config(&temp_dir);
    variation.apply_to_config(&mut config);

    let executor = if variation.quiet {
        let mut mock_executor = MockCommandExecutor::new();
        mock_executor
            .expect_bootc()
            .times(2) // Version check + upgrade
            .returning(|args| {
                if args.contains(&"--version".to_string()) {
                    Ok(create_success_output("bootc 1.0.0"))
                } else {
                    Ok(create_success_output("Upgrade completed"))
                }
            });
        Arc::new(mock_executor)
    } else {
        Arc::new(MockScenarios::all_success())
    };

    let runner = ContainerRunner::new(&config, executor);
    let result = runner.run_bootc_upgrade();
    assert!(result.is_ok());
}

#[test]
fn test_run_bootc_upgrade_success_standard() {
    test_run_bootc_upgrade_success_impl(TestVariation::standard());
}

#[test]
fn test_run_bootc_upgrade_success_quiet() {
    test_run_bootc_upgrade_success_impl(TestVariation::quiet());
}

fn test_run_container_failure_impl(variation: TestVariation) {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_runner_config(&temp_dir);
    variation.apply_to_config(&mut config);

    let executor = if variation.quiet {
        let mut mock_executor = MockCommandExecutor::new();
        mock_executor
            .expect_execute()
            .returning(|_, _| Ok(create_success_output("Image exists")));
        mock_executor
            .expect_podman_run()
            .returning(|_| Ok(create_failure_output("Run failed")));
        Arc::new(mock_executor)
    } else {
        let mut mock_executor = MockCommandExecutor::new();
        mock_executor
            .expect_execute()
            .with(
                mockall::predicate::eq("podman"),
                mockall::predicate::function(|args: &[String]| {
                    args.len() >= 3 && args[0] == "image" && args[1] == "exists"
                }),
            )
            .returning(|_, _| Ok(create_success_output("Image exists")));
        mock_executor
            .expect_podman_run_streaming()
            .returning(|_| Ok(create_failure_status()));
        Arc::new(mock_executor)
    };

    let runner = ContainerRunner::new(&config, executor);
    let args = vec!["echo".to_string(), "hello".to_string()];
    let result = runner.run_container("test-rootfs", &args);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Podman run failed"));
}

#[test]
fn test_run_container_failure_standard() {
    test_run_container_failure_impl(TestVariation::standard());
}

#[test]
fn test_run_container_failure_quiet() {
    test_run_container_failure_impl(TestVariation::quiet());
}

fn test_run_bootc_upgrade_failure_impl(variation: TestVariation) {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_runner_config(&temp_dir);
    variation.apply_to_config(&mut config);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_bootc()
        .with(mockall::predicate::function(|args: &[String]| {
            args.len() == 1 && args[0] == "--version"
        }))
        .returning(|_| Ok(create_success_output("bootc 1.0.0")));

    if variation.quiet {
        mock_executor
            .expect_bootc()
            .with(mockall::predicate::function(|args: &[String]| {
                args.len() == 1 && args[0] == "upgrade"
            }))
            .returning(|_| Ok(create_failure_output("Upgrade failed")));
    } else {
        mock_executor
            .expect_bootc_streaming()
            .with(mockall::predicate::function(|args: &[String]| {
                args.len() == 1 && args[0] == "upgrade"
            }))
            .returning(|_| Ok(create_failure_status()));
    }

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);
    let result = runner.run_bootc_upgrade();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("bootc upgrade failed"));
}

#[test]
fn test_run_bootc_upgrade_failure_standard() {
    test_run_bootc_upgrade_failure_impl(TestVariation::standard());
}

#[test]
fn test_run_bootc_upgrade_failure_quiet() {
    test_run_bootc_upgrade_failure_impl(TestVariation::quiet());
}

#[test]
fn test_run_bootc_upgrade_bootc_not_available() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_bootc()
        .with(mockall::predicate::function(|args: &[String]| {
            args.len() == 1 && args[0] == "--version"
        }))
        .returning(|_| Err(anyhow::anyhow!("bootc command not found")));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let result = runner.run_bootc_upgrade();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("bootc is not available"));
}

#[test]
fn test_run_bootc_upgrade_upgrade_failure() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    // Version check succeeds
    mock_executor
        .expect_bootc()
        .with(mockall::predicate::function(|args: &[String]| {
            args.len() == 1 && args[0] == "--version"
        }))
        .returning(|_| Ok(create_success_output("bootc 1.0.0")));

    // But upgrade fails
    mock_executor
        .expect_bootc_streaming()
        .with(mockall::predicate::function(|args: &[String]| {
            args.len() == 1 && args[0] == "upgrade"
        }))
        .returning(|_| Ok(create_failure_status()));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let result = runner.run_bootc_upgrade();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("bootc upgrade failed"));
}

#[test]
fn test_container_capability_enum() {
    // Test that the enum exists and can be used
    let _capability = ContainerCapability::All;
    // Note: as_str() is private, so we just test the enum exists
}

#[test]
fn test_podman_run_command_builder_new() {
    let _builder = PodmanRunCommandBuilder::new();
    // Test passes if no panic occurs during creation
}

#[test]
fn test_podman_run_command_builder_default() {
    let _builder = PodmanRunCommandBuilder::default();
    // Test passes if no panic occurs during creation
}

#[test]
fn test_podman_run_command_builder_chaining() {
    let _builder = PodmanRunCommandBuilder::new()
        .network_host()
        .add_capability(ContainerCapability::All)
        .remove_on_exit()
        .interactive()
        .image("test-image")
        .args(&["echo".to_string(), "hello".to_string()]);
    // Test passes if no panic occurs during chaining
}

#[test]
fn test_container_validation_success() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_execute()
        .with(
            mockall::predicate::eq("podman"),
            mockall::predicate::function(|args: &[String]| {
                args.len() >= 3 && args[0] == "image" && args[1] == "exists"
            }),
        )
        .returning(|_, _| Ok(create_success_output("Image exists")));

    mock_executor
        .expect_podman_run_streaming()
        .returning(|_| Ok(create_success_status()));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let args = vec!["echo".to_string(), "test".to_string()];
    let result = runner.run_container("test-rootfs", &args);
    assert!(result.is_ok());
}

#[test]
fn test_container_validation_failure() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_execute()
        .with(
            mockall::predicate::eq("podman"),
            mockall::predicate::function(|args: &[String]| {
                args.len() >= 3 && args[0] == "image" && args[1] == "exists"
            }),
        )
        .returning(|_, _| Ok(create_failure_output("Image not found")));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let args = vec!["echo".to_string(), "test".to_string()];
    let result = runner.run_container("missing-image", &args);
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Container image not found"));
    assert!(error_message.contains("Run 'trls build' first"));
}

#[test]
fn test_bootc_validation_success() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_bootc()
        .with(mockall::predicate::function(|args: &[String]| {
            args.len() == 1 && args[0] == "--version"
        }))
        .returning(|_| Ok(create_success_output("bootc 1.0.0")));

    mock_executor
        .expect_bootc_streaming()
        .returning(|_| Ok(create_success_status()));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let result = runner.run_bootc_upgrade();
    assert!(result.is_ok());
}

#[test]
fn test_bootc_validation_failure() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_bootc()
        .with(mockall::predicate::function(|args: &[String]| {
            args.len() == 1 && args[0] == "--version"
        }))
        .returning(|_| Err(anyhow::anyhow!("Command not found")));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let result = runner.run_bootc_upgrade();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("bootc is not available"));
}

#[test]
fn test_run_container_with_localhost_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();

    // Verify that the image exists check uses the correct localhost prefix
    mock_executor
        .expect_execute()
        .with(
            mockall::predicate::eq("podman"),
            mockall::predicate::function(|args: &[String]| {
                args.len() >= 3
                    && args[0] == "image"
                    && args[1] == "exists"
                    && args[2].starts_with("localhost/")
            }),
        )
        .returning(|_, _| Ok(create_success_output("Image exists")));

    // Verify that the run command uses the correct localhost prefix
    mock_executor
        .expect_podman_run_streaming()
        .with(mockall::predicate::function(|args: &[String]| {
            args.iter().any(|arg| arg.starts_with("localhost/"))
        }))
        .returning(|_| Ok(create_success_status()));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let args = vec!["echo".to_string(), "test".to_string()];
    let result = runner.run_container("test-rootfs", &args);
    assert!(result.is_ok());
}

#[test]
fn test_run_container_error_message_includes_build_hint() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_runner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_execute()
        .returning(|_, _| Ok(create_failure_output("Image not found")));

    let executor = Arc::new(mock_executor);
    let runner = ContainerRunner::new(&config, executor);

    let args = vec!["echo".to_string(), "test".to_string()];
    let result = runner.run_container("missing-image", &args);
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Container image not found"));
    assert!(error_message.contains("Run 'trls build' first"));
}
