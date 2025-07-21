//! Integration tests for command execution functionality.
//!
//! Tests cover executor trait implementation, command argument handling, and integration patterns.

mod common;

use common::mocks::*;
use std::sync::Arc;
use trellis::trellis::executor::{CommandExecutor, RealCommandExecutor};

#[test]
fn test_real_command_executor_creation() {
    let _executor = RealCommandExecutor::new();
    // Test passes if no panic occurs during creation
}

#[test]
fn test_real_command_executor_default() {
    let _executor = RealCommandExecutor;
    // Test passes if no panic occurs during creation
}

#[test]
fn test_mock_command_executor_creation() {
    let _executor = MockCommandExecutor::new();
    // Test passes if no panic occurs during creation
}

#[test]
fn test_mock_executor_builder_creation() {
    let _builder = MockCommandExecutorBuilder::new();
    // Test passes if no panic occurs during creation
}

#[test]
fn test_mock_executor_builder_default() {
    let _builder = MockCommandExecutorBuilder::default();
    // Test passes if no panic occurs during creation
}

#[test]
fn test_mock_executor_builder_with_successful_builds() {
    let mock = MockCommandExecutorBuilder::new()
        .with_successful_builds(&["base", "tools", "final"])
        .build();

    // Test that configured builds work via trait
    let executor: &dyn CommandExecutor = &mock;
    let result =
        executor.podman_build(&["--tag".to_string(), "test".to_string(), "base".to_string()]);
    assert!(result.is_ok());
    assert!(result.unwrap().status.success());
}

#[test]
fn test_mock_executor_builder_with_successful_runs() {
    let mock = MockCommandExecutorBuilder::new()
        .with_successful_runs(&["test-image", "another-image"])
        .build();

    let executor: &dyn CommandExecutor = &mock;
    let result = executor.podman_run(&[
        "test-image".to_string(),
        "echo".to_string(),
        "hello".to_string(),
    ]);
    assert!(result.is_ok());
    assert!(result.unwrap().status.success());
}

#[test]
fn test_mock_executor_builder_with_images() {
    let images = vec![
        MockImageInfo::new("abc123", "localhost/test", "latest"),
        MockImageInfo::new("def456", "localhost/other", "v1.0"),
    ];

    let mock = MockCommandExecutorBuilder::new()
        .with_images(images)
        .build();

    let executor: &dyn CommandExecutor = &mock;
    let result = executor.podman_images(&[
        "--format".to_string(),
        "{{.Repository}}:{{.Tag}}".to_string(),
    ]);
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("localhost/test"));
    assert!(stdout.contains("localhost/other"));
}

#[test]
fn test_mock_executor_builder_with_successful_rmi() {
    let mock = MockCommandExecutorBuilder::new()
        .with_successful_rmi()
        .build();

    let executor: &dyn CommandExecutor = &mock;
    let result = executor.podman_rmi(&["image1".to_string(), "image2".to_string()]);
    assert!(result.is_ok());
    assert!(result.unwrap().status.success());
}

#[test]
fn test_mock_executor_builder_with_bootc_support() {
    let mock = MockCommandExecutorBuilder::new()
        .with_bootc_support()
        .build();

    let executor: &dyn CommandExecutor = &mock;

    // Test version check
    let version_result = executor.bootc(&["--version".to_string()]);
    assert!(version_result.is_ok());
    assert!(version_result.unwrap().status.success());

    // Test upgrade command
    let upgrade_result = executor.bootc(&["upgrade".to_string()]);
    assert!(upgrade_result.is_ok());
    assert!(upgrade_result.unwrap().status.success());
}

#[test]
fn test_mock_executor_builder_with_build_failures() {
    let mock = MockCommandExecutorBuilder::new()
        .with_build_failures(&["failing-stage"])
        .build();

    let executor: &dyn CommandExecutor = &mock;
    let result = executor.podman_build(&[
        "--tag".to_string(),
        "test".to_string(),
        "failing-stage".to_string(),
    ]);
    assert!(result.is_ok());
    assert!(!result.unwrap().status.success());
}

#[test]
fn test_mock_executor_builder_chaining() {
    let mock = MockCommandExecutorBuilder::new()
        .with_successful_builds(&["base", "tools"])
        .with_successful_runs(&["test-image"])
        .with_images(vec![MockImageInfo::new(
            "abc123",
            "localhost/test",
            "latest",
        )])
        .with_successful_rmi()
        .with_bootc_support()
        .build();

    let executor: &dyn CommandExecutor = &mock;

    // Test that all configured functionality works
    let build_result = executor.podman_build(&["base".to_string()]);
    assert!(build_result.is_ok() && build_result.unwrap().status.success());

    let run_result = executor.podman_run(&["test-image".to_string()]);
    assert!(run_result.is_ok() && run_result.unwrap().status.success());

    let images_result = executor.podman_images(&[]);
    assert!(images_result.is_ok() && images_result.unwrap().status.success());

    let rmi_result = executor.podman_rmi(&["test".to_string()]);
    assert!(rmi_result.is_ok() && rmi_result.unwrap().status.success());

    let bootc_result = executor.bootc(&["--version".to_string()]);
    assert!(bootc_result.is_ok() && bootc_result.unwrap().status.success());
}

#[test]
fn test_mock_scenarios_all_success() {
    let mock = MockScenarios::all_success();
    let executor: &dyn CommandExecutor = &mock;

    let build_result = executor.podman_build(&["base".to_string()]);
    assert!(build_result.is_ok() && build_result.unwrap().status.success());

    let run_result = executor.podman_run(&["test-builder".to_string()]);
    assert!(run_result.is_ok() && run_result.unwrap().status.success());
}

#[test]
fn test_mock_scenarios_build_failures() {
    let mock = MockScenarios::build_failures();
    let executor: &dyn CommandExecutor = &mock;
    let result = executor.podman_build(&["builder".to_string()]);
    assert!(result.is_ok());
    assert!(!result.unwrap().status.success());
}

#[test]
fn test_mock_scenarios_no_images() {
    let mock = MockScenarios::no_images();
    let executor: &dyn CommandExecutor = &mock;
    let result = executor.podman_images(&[]);
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Should only contain header
    assert!(stdout.starts_with("REPOSITORY"));
    assert_eq!(stdout.lines().count(), 1);
}

#[test]
fn test_mock_scenarios_multiple_images() {
    let mock = MockScenarios::multiple_images();
    let executor: &dyn CommandExecutor = &mock;
    let result = executor.podman_images(&[]);
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("localhost/test-builder"));
    assert!(stdout.contains("localhost/test-rootfs"));
    assert!(stdout.contains("intermediate"));
}

#[test]
fn test_test_environment_creation() {
    let env = TestEnvironment::new();
    assert!(env.temp_dir.path().exists());
}

#[test]
fn test_test_environment_with_custom_executor() {
    let custom_mock = MockCommandExecutorBuilder::new()
        .with_successful_builds(&["custom"])
        .build();

    let env = TestEnvironment::with_executor(custom_mock);
    assert!(env.temp_dir.path().exists());
}

#[test]
fn test_command_executor_trait_methods() {
    let mut mock = MockCommandExecutor::new();

    // Set up expectations for empty argument calls
    mock.expect_podman_build()
        .returning(|_| Ok(create_success_output("Build success")));
    mock.expect_podman_run()
        .returning(|_| Ok(create_success_output("Run success")));
    mock.expect_podman_images()
        .returning(|_| Ok(create_success_output("Images success")));
    mock.expect_podman_rmi()
        .returning(|_| Ok(create_success_output("RMI success")));
    mock.expect_bootc()
        .returning(|_| Ok(create_success_output("Bootc success")));
    mock.expect_execute()
        .returning(|_, _| Ok(create_success_output("Execute success")));

    let executor: &dyn CommandExecutor = &mock;

    // Test all trait methods are callable
    let _build_result = executor.podman_build(&[]);
    let _run_result = executor.podman_run(&[]);
    let _images_result = executor.podman_images(&[]);
    let _rmi_result = executor.podman_rmi(&[]);
    let _bootc_result = executor.bootc(&[]);
    let _execute_result = executor.execute("echo", &["hello".to_string()]);
}

#[test]
fn test_create_success_output_utility() {
    let output = create_success_output("test message");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "test message");
    assert!(output.stderr.is_empty());
}

#[test]
fn test_create_failure_output_utility() {
    let output = create_failure_output("error message");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "error message");
}

#[test]
fn test_mock_image_info_creation() {
    let image = MockImageInfo::new("abc123", "localhost/test", "latest");
    assert_eq!(image.id, "abc123");
    assert_eq!(image.repository, "localhost/test");
    assert_eq!(image.tag, "latest");
    assert_eq!(image.created, "2024-01-01T00:00:00Z");
    assert_eq!(image.size, "100MB");
}

#[test]
fn test_mock_image_info_clone() {
    let image1 = MockImageInfo::new("abc123", "localhost/test", "latest");
    let image2 = image1.clone();
    assert_eq!(image1, image2);
}

#[test]
fn test_arc_executor_usage() {
    let mut mock = MockCommandExecutor::new();

    // Set up expectations for the specific "test" arguments
    mock.expect_podman_build()
        .with(mockall::predicate::function(|args: &[String]| {
            args.len() == 1 && args[0] == "test"
        }))
        .returning(|_| Ok(create_success_output("Build success")));

    mock.expect_podman_run()
        .with(mockall::predicate::function(|args: &[String]| {
            args.len() == 1 && args[0] == "test"
        }))
        .returning(|_| Ok(create_success_output("Run success")));

    let executor = Arc::new(mock);
    let executor_clone = Arc::clone(&executor);

    // Test that Arc<dyn CommandExecutor> works properly
    let result1 = executor.podman_build(&["test".to_string()]);
    let result2 = executor_clone.podman_run(&["test".to_string()]);

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[test]
fn test_executor_error_handling() {
    let mut mock = MockCommandExecutor::new();
    mock.expect_podman_build()
        .returning(|_| Err(anyhow::anyhow!("Build command failed")));

    let executor: &dyn CommandExecutor = &mock;
    let result = executor.podman_build(&["test".to_string()]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Build command failed"));
}

#[test]
fn test_executor_argument_matching() {
    let mut mock = MockCommandExecutor::new();

    // Set up specific expectations based on arguments
    mock.expect_podman_build()
        .with(mockall::predicate::function(|args: &[String]| {
            args.iter().any(|arg| arg.contains("specific-stage"))
        }))
        .returning(|_| Ok(create_success_output("Specific stage built")));

    mock.expect_podman_build()
        .with(mockall::predicate::function(|args: &[String]| {
            !args.iter().any(|arg| arg.contains("specific-stage"))
        }))
        .returning(|_| Ok(create_failure_output("Other stages fail")));

    let executor: &dyn CommandExecutor = &mock;

    // Test specific stage succeeds
    let specific_result = executor.podman_build(&[
        "--tag".to_string(),
        "test".to_string(),
        "specific-stage".to_string(),
    ]);
    assert!(specific_result.is_ok() && specific_result.unwrap().status.success());

    // Test other stage fails
    let other_result = executor.podman_build(&[
        "--tag".to_string(),
        "test".to_string(),
        "other-stage".to_string(),
    ]);
    assert!(other_result.is_ok() && !other_result.unwrap().status.success());
}

#[test]
fn test_executor_with_complex_arguments() {
    let mock = MockScenarios::all_success();
    let executor: &dyn CommandExecutor = &mock;

    // Test with complex argument patterns
    let complex_args = vec![
        "--no-cache".to_string(),
        "--tag".to_string(),
        "localhost/test:latest".to_string(),
        "--build-arg".to_string(),
        "BASE_IMAGE=fedora:39".to_string(),
        "--file".to_string(),
        "Containerfile.base".to_string(),
        ".".to_string(),
    ];

    let result = executor.podman_build(&complex_args);
    assert!(result.is_ok());
    assert!(result.unwrap().status.success());
}

#[test]
fn test_executor_empty_arguments() {
    let mock = MockScenarios::all_success();
    let executor: &dyn CommandExecutor = &mock;

    // Test with empty arguments
    let result = executor.podman_images(&[]);
    assert!(result.is_ok());
    assert!(result.unwrap().status.success());
}

#[test]
fn test_executor_unicode_arguments() {
    let mut mock = MockCommandExecutor::new();

    // Set up expectations for unicode arguments
    mock.expect_podman_build()
        .with(mockall::predicate::function(|args: &[String]| {
            args.len() == 2 && args[0] == "--tag" && args[1] == "测试镜像:latest"
        }))
        .returning(|_| Ok(create_success_output("Unicode build success")));

    let executor: &dyn CommandExecutor = &mock;

    // Test with unicode arguments
    let unicode_args = vec!["--tag".to_string(), "测试镜像:latest".to_string()];

    let result = executor.podman_build(&unicode_args);
    assert!(result.is_ok());
    assert!(result.unwrap().status.success());
}
