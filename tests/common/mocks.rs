//! Complete mock infrastructure for testing external dependencies.
//!
//! This module provides comprehensive mocking capabilities for all external
//! dependencies, particularly Podman commands and filesystem operations.

use anyhow::Result;
use mockall::predicate::*;
use mockall::*;
use std::process::{ExitStatus, Output};

// Re-export the trait and real implementation for testing
pub use trellis::trellis::executor::{CommandExecutor, RealCommandExecutor};

/// Mock image information for testing.
#[derive(Debug, Clone, PartialEq)]
pub struct MockImageInfo {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub created: String,
    pub size: String,
}

impl MockImageInfo {
    pub fn new(id: &str, repository: &str, tag: &str) -> Self {
        Self {
            id: id.to_string(),
            repository: repository.to_string(),
            tag: tag.to_string(),
            created: "2024-01-01T00:00:00Z".to_string(),
            size: "100MB".to_string(),
        }
    }
}

// Traits and types are already re-exported above

// Generate mock for the trait
mock! {
    pub CommandExecutor {}
    
    impl CommandExecutor for CommandExecutor {
        fn podman_build(&self, args: &[String]) -> Result<Output>;
        fn podman_run(&self, args: &[String]) -> Result<Output>;
        fn podman_images(&self, args: &[String]) -> Result<Output>;
        fn podman_rmi(&self, args: &[String]) -> Result<Output>;
        fn bootc(&self, args: &[String]) -> Result<Output>;
        fn execute(&self, command: &str, args: &[String]) -> Result<Output>;
    }
}

/// Mock command executor builder for easy test setup.
pub struct MockCommandExecutorBuilder {
    mock: MockCommandExecutor,
}

impl MockCommandExecutorBuilder {
    pub fn new() -> Self {
        Self {
            mock: MockCommandExecutor::new(),
        }
    }

    /// Configure successful podman build responses.
    pub fn with_successful_builds(mut self, commands: &[&str]) -> Self {
        for command in commands {
            let command_owned = command.to_string();
            self.mock
                .expect_podman_build()
                .with(predicate::function(move |args: &[String]| {
                    args.iter().any(|arg| arg.contains(&command_owned))
                }))
                .returning(|_| Ok(create_success_output("Build completed successfully")));
        }
        self
    }

    /// Configure successful podman run responses.
    pub fn with_successful_runs(mut self, tags: &[&str]) -> Self {
        for tag in tags {
            let tag_owned = tag.to_string();
            self.mock
                .expect_podman_run()
                .with(predicate::function(move |args: &[String]| {
                    args.iter().any(|arg| arg.contains(&tag_owned))
                }))
                .returning(|_| Ok(create_success_output("Container executed successfully")));
        }
        self
    }

    /// Configure image listing responses.
    pub fn with_images(mut self, images: Vec<MockImageInfo>) -> Self {
        let images_output = format_images_output(&images);
        self.mock
            .expect_podman_images()
            .returning(move |_| Ok(create_success_output(&images_output)));
        self
    }

    /// Configure successful image removal.
    pub fn with_successful_rmi(mut self) -> Self {
        self.mock
            .expect_podman_rmi()
            .returning(|_| Ok(create_success_output("Image removed successfully")));
        self
    }

    /// Configure bootc command responses.
    pub fn with_bootc_support(mut self) -> Self {
        self.mock.expect_bootc().returning(|args| {
            if args.contains(&"--version".to_string()) {
                Ok(create_success_output("bootc 1.0.0"))
            } else if args.contains(&"upgrade".to_string()) {
                Ok(create_success_output("Upgrade completed successfully"))
            } else {
                Ok(create_success_output("bootc command executed"))
            }
        });
        self
    }

    /// Configure build failures for specific commands.
    pub fn with_build_failures(mut self, failing_commands: &[&str]) -> Self {
        for command in failing_commands {
            let command_owned = command.to_string();
            self.mock
                .expect_podman_build()
                .with(predicate::function(move |args: &[String]| {
                    args.iter().any(|arg| arg.contains(&command_owned))
                }))
                .returning(|_| Ok(create_failure_output("Build failed")));
        }
        self
    }

    pub fn build(self) -> MockCommandExecutor {
        self.mock
    }
}

impl Default for MockCommandExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create successful command output.
pub fn create_success_output(stdout: &str) -> Output {
    Output {
        status: create_success_status(),
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    }
}

/// Helper function to create failure command output.
pub fn create_failure_output(stderr: &str) -> Output {
    Output {
        status: create_failure_status(),
        stdout: Vec::new(),
        stderr: stderr.as_bytes().to_vec(),
    }
}

/// Helper function to create success exit status.
fn create_success_status() -> ExitStatus {
    // This is a bit hacky but works for testing
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(0)
    }
    #[cfg(not(unix))]
    {
        // For non-Unix systems, we need a different approach
        // This will create a successful status for testing
        std::process::Command::new("true").status().unwrap()
    }
}

/// Helper function to create failure exit status.
fn create_failure_status() -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(256) // Exit code 1
    }
    #[cfg(not(unix))]
    {
        // For non-Unix systems
        std::process::Command::new("false").status().unwrap()
    }
}

/// Format mock images for podman images output.
fn format_images_output(images: &[MockImageInfo]) -> String {
    let mut output = String::from("REPOSITORY\tTAG\tIMAGE ID\tCREATED\tSIZE\n");
    for image in images {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            image.repository, image.tag, image.id, image.created, image.size
        ));
    }
    output
}

/// Test environment setup utilities.
pub struct TestEnvironment {
    pub temp_dir: tempfile::TempDir,
    pub mock_executor: MockCommandExecutor,
}

impl TestEnvironment {
    /// Create a complete test environment with mocked executor.
    pub fn new() -> Self {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let mock_executor = MockCommandExecutorBuilder::new()
            .with_successful_builds(&["builder", "stage"])
            .with_successful_runs(&["test-builder", "test-rootfs"])
            .with_images(vec![
                MockImageInfo::new("abc123", "localhost/test-builder", "latest"),
                MockImageInfo::new("def456", "localhost/test-rootfs", "latest"),
            ])
            .with_successful_rmi()
            .with_bootc_support()
            .build();

        Self {
            temp_dir,
            mock_executor,
        }
    }

    /// Create environment with specific executor configuration.
    pub fn with_executor(mock_executor: MockCommandExecutor) -> Self {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        Self {
            temp_dir,
            mock_executor,
        }
    }
}

/// Collection of pre-configured mock scenarios for common test cases.
pub struct MockScenarios;

impl MockScenarios {
    /// Scenario: All operations succeed.
    pub fn all_success() -> MockCommandExecutor {
        MockCommandExecutorBuilder::new()
            .with_successful_builds(&["builder", "stage", "base", "tools", "final"])
            .with_successful_runs(&["test-builder", "test-rootfs"])
            .with_images(vec![
                MockImageInfo::new("abc123", "localhost/test-builder", "latest"),
                MockImageInfo::new("def456", "localhost/test-rootfs", "latest"),
            ])
            .with_successful_rmi()
            .with_bootc_support()
            .build()
    }

    /// Scenario: Build operations fail.
    pub fn build_failures() -> MockCommandExecutor {
        MockCommandExecutorBuilder::new()
            .with_build_failures(&["builder", "stage"])
            .build()
    }

    /// Scenario: No images exist.
    pub fn no_images() -> MockCommandExecutor {
        MockCommandExecutorBuilder::new()
            .with_images(vec![])
            .build()
    }

    /// Scenario: Multiple images need cleanup.
    pub fn multiple_images() -> MockCommandExecutor {
        MockCommandExecutorBuilder::new()
            .with_images(vec![
                MockImageInfo::new("abc123", "localhost/test-builder", "latest"),
                MockImageInfo::new("def456", "localhost/test-rootfs", "latest"),
                MockImageInfo::new("ghi789", "localhost/test-builder", "intermediate"),
                MockImageInfo::new("jkl012", "localhost/test-rootfs", "intermediate"),
            ])
            .with_successful_rmi()
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_image_info_creation() {
        let image = MockImageInfo::new("abc123", "localhost/test", "latest");
        assert_eq!(image.id, "abc123");
        assert_eq!(image.repository, "localhost/test");
        assert_eq!(image.tag, "latest");
    }

    #[test]
    fn test_success_output_creation() {
        let output = create_success_output("test output");
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "test output");
    }

    #[test]
    fn test_failure_output_creation() {
        let output = create_failure_output("test error");
        assert!(!output.status.success());
        assert_eq!(String::from_utf8(output.stderr).unwrap(), "test error");
    }

    #[test]
    fn test_images_output_formatting() {
        let images = vec![
            MockImageInfo::new("abc123", "localhost/test", "latest"),
            MockImageInfo::new("def456", "localhost/other", "v1.0"),
        ];
        let output = format_images_output(&images);
        assert!(output.contains("REPOSITORY"));
        assert!(output.contains("localhost/test"));
        assert!(output.contains("localhost/other"));
    }

    #[test]
    fn test_test_environment_creation() {
        let env = TestEnvironment::new();
        assert!(env.temp_dir.path().exists());
    }

    #[test]
    fn test_mock_scenarios() {
        let _all_success = MockScenarios::all_success();
        let _build_failures = MockScenarios::build_failures();
        let _no_images = MockScenarios::no_images();
        let _multiple_images = MockScenarios::multiple_images();
        // Test passes if no panics occur during mock creation
    }
}
