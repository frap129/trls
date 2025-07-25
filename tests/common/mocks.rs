//! Complete mock infrastructure for testing external dependencies.
//!
//! This module provides comprehensive mocking capabilities for all external
//! dependencies, particularly Podman commands and filesystem operations.

use anyhow::Result;
use mockall::predicate::*;
use mockall::*;
use std::process::{ExitStatus, Output};

// Re-export the trait and real implementation for testing
pub use trellis::trellis::executor::CommandExecutor;
pub use trellis::trellis::UserInteraction;

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
        fn podman_build_streaming(&self, args: &[String]) -> Result<ExitStatus>;
        fn podman_run(&self, args: &[String]) -> Result<Output>;
        fn podman_run_streaming(&self, args: &[String]) -> Result<ExitStatus>;
        fn podman_images(&self, args: &[String]) -> Result<Output>;
        fn podman_rmi(&self, args: &[String]) -> Result<Output>;
        fn bootc(&self, args: &[String]) -> Result<Output>;
        fn bootc_streaming(&self, args: &[String]) -> Result<ExitStatus>;
        fn execute(&self, command: &str, args: &[String]) -> Result<Output>;
    }
}

// Generate mock for the UserInteraction trait
mock! {
    pub UserInteraction {}

    impl UserInteraction for UserInteraction {
        fn prompt_yes_no(&self, message: &str) -> Result<bool>;
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

    /// Configure generic execute command support for image validation, etc.
    #[allow(dead_code)]
    pub fn with_execute_support(mut self) -> Self {
        self.mock.expect_execute().returning(|command, args| {
            if command == "podman" && args.len() >= 2 && args[0] == "image" && args[1] == "exists" {
                // Image exists check - return success
                Ok(create_success_output(""))
            } else {
                // Generic success for other execute calls
                Ok(create_success_output("Command executed successfully"))
            }
        });
        self
    }

    /// Configure build failures for specific commands.
    #[allow(dead_code)]
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
pub fn create_success_status() -> ExitStatus {
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
pub fn create_failure_status() -> ExitStatus {
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

/// Collection of pre-configured user interaction mock scenarios.
#[allow(dead_code)]
pub struct MockUserInteractionScenarios;

impl MockScenarios {
    /// Scenario: All operations succeed.
    pub fn all_success() -> MockCommandExecutor {
        let mut mock = MockCommandExecutor::new();

        // Accept any podman build command (multiple times)
        mock.expect_podman_build()
            .times(..)
            .returning(|_| Ok(create_success_output("Build completed successfully")));

        // Accept any podman build streaming command (multiple times)
        mock.expect_podman_build_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        // Accept any podman run command (multiple times)
        mock.expect_podman_run()
            .times(..)
            .returning(|_| Ok(create_success_output("Container executed successfully")));

        // Accept any podman run streaming command (multiple times)
        mock.expect_podman_run_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        // Accept any podman images command (multiple times)
        // Return appropriate format based on the arguments
        mock.expect_podman_images()
            .times(..)
            .returning(|args| {
                // Check if this is a builder container existence check
                if args.iter().any(|arg| arg.contains("--filter")) && 
                   args.iter().any(|arg| arg.contains("reference=localhost/test-builder")) {
                    // Return the expected format for builder container check
                    Ok(create_success_output("localhost/test-builder:latest\n"))
                } else {
                    // Return tabular format for general image listing
                    Ok(create_success_output("REPOSITORY\tTAG\tIMAGE ID\tCREATED\tSIZE\nlocalhost/test-builder\tlatest\tabc123\t2024-01-01T00:00:00Z\t100MB\nlocalhost/test-rootfs\tlatest\tdef456\t2024-01-01T00:00:00Z\t100MB\n"))
                }
            });

        // Accept any podman rmi command (multiple times)
        mock.expect_podman_rmi()
            .times(..)
            .returning(|_| Ok(create_success_output("Image removed successfully")));

        // Accept any bootc command (multiple times)
        mock.expect_bootc().times(..).returning(|args| {
            if args.contains(&"--version".to_string()) {
                Ok(create_success_output("bootc 1.0.0"))
            } else if args.contains(&"upgrade".to_string()) {
                Ok(create_success_output("Upgrade completed successfully"))
            } else {
                Ok(create_success_output("bootc command executed"))
            }
        });

        // Accept any execute command (multiple times)
        mock.expect_execute().times(..).returning(|command, args| {
            if command == "podman" && args.len() >= 2 && args[0] == "image" && args[1] == "exists" {
                Ok(create_success_output(""))
            } else {
                Ok(create_success_output("Command executed successfully"))
            }
        });

        // Accept streaming methods
        mock.expect_podman_build_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        mock.expect_podman_run_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        mock.expect_bootc_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        mock
    }

    /// Scenario: Build operations fail.
    pub fn build_failures() -> MockCommandExecutor {
        let mut mock = MockCommandExecutor::new();

        // Accept any podman build command and return failure (multiple times)
        mock.expect_podman_build()
            .times(..)
            .returning(|_| Ok(create_failure_output("Build failed")));

        // Accept podman build streaming and return failure
        mock.expect_podman_build_streaming()
            .times(..)
            .returning(|_| Ok(create_failure_status()));

        // Accept other commands with flexible expectations
        mock.expect_podman_run()
            .times(..)
            .returning(|_| Ok(create_success_output("Container executed successfully")));

        mock.expect_podman_run_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        mock.expect_podman_images().times(..).returning(|args| {
            // Check if this is a builder container existence check
            if args.iter().any(|arg| arg.contains("--filter"))
                && args
                    .iter()
                    .any(|arg| arg.contains("reference=localhost/test-builder"))
            {
                // Return the expected format for builder container check
                Ok(create_success_output("localhost/test-builder:latest\n"))
            } else {
                // Return empty list for other calls
                Ok(create_success_output(
                    "REPOSITORY\tTAG\tIMAGE ID\tCREATED\tSIZE\n",
                ))
            }
        });

        mock.expect_podman_rmi()
            .times(..)
            .returning(|_| Ok(create_success_output("Image removed successfully")));

        mock.expect_bootc().times(..).returning(|args| {
            if args.contains(&"--version".to_string()) {
                Ok(create_success_output("bootc 1.0.0"))
            } else {
                Ok(create_success_output("bootc command executed"))
            }
        });

        mock.expect_bootc_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        mock.expect_execute().times(..).returning(|command, args| {
            if command == "podman" && args.len() >= 2 && args[0] == "image" && args[1] == "exists" {
                Ok(create_success_output(""))
            } else {
                Ok(create_success_output("Command executed successfully"))
            }
        });

        mock
    }

    /// Scenario: No images exist.
    pub fn no_images() -> MockCommandExecutor {
        MockCommandExecutorBuilder::new()
            .with_images(vec![])
            .build()
    }

    /// Scenario: Multiple images need cleanup.
    pub fn multiple_images() -> MockCommandExecutor {
        let mut mock = MockCommandExecutor::new();

        // Accept any podman build command (multiple times)
        mock.expect_podman_build()
            .times(..)
            .returning(|_| Ok(create_success_output("Build completed successfully")));

        mock.expect_podman_build_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        // Accept any podman run command (multiple times)
        mock.expect_podman_run()
            .times(..)
            .returning(|_| Ok(create_success_output("Container executed successfully")));

        mock.expect_podman_run_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        // Return multiple images for cleanup and handle builder container check
        mock.expect_podman_images()
            .times(..)
            .returning(|args| {
                // Check if this is a builder container existence check
                if args.iter().any(|arg| arg.contains("--filter")) && 
                   args.iter().any(|arg| arg.contains("reference=localhost/test-builder")) {
                    // Return the expected format for builder container check
                    Ok(create_success_output("localhost/test-builder:latest\n"))
                } else {
                    // Return multiple images for cleanup testing
                    Ok(create_success_output("REPOSITORY\tTAG\tIMAGE ID\tCREATED\tSIZE\nlocalhost/test-builder\tlatest\tabc123\t2024-01-01T00:00:00Z\t100MB\nlocalhost/test-rootfs\tlatest\tdef456\t2024-01-01T00:00:00Z\t100MB\nlocalhost/test-builder\tintermediate\tghi789\t2024-01-01T00:00:00Z\t100MB\nlocalhost/test-rootfs\tintermediate\tjkl012\t2024-01-01T00:00:00Z\t100MB\n"))
                }
            });

        // Accept any podman rmi command (multiple times)
        mock.expect_podman_rmi()
            .times(..)
            .returning(|_| Ok(create_success_output("Image removed successfully")));

        // Accept any bootc command (multiple times)
        mock.expect_bootc().times(..).returning(|args| {
            if args.contains(&"--version".to_string()) {
                Ok(create_success_output("bootc 1.0.0"))
            } else if args.contains(&"upgrade".to_string()) {
                Ok(create_success_output("Upgrade completed successfully"))
            } else {
                Ok(create_success_output("bootc command executed"))
            }
        });

        // Accept any execute command (multiple times)
        mock.expect_execute().times(..).returning(|command, args| {
            if command == "podman" && args.len() >= 2 && args[0] == "image" && args[1] == "exists" {
                Ok(create_success_output(""))
            } else {
                Ok(create_success_output("Command executed successfully"))
            }
        });

        mock.expect_bootc_streaming()
            .times(..)
            .returning(|_| Ok(create_success_status()));

        mock
    }
}

/// Helper function to create a default user interaction mock for standard tests.
/// This creates a mock that expects never to be called, suitable for tests where
/// the builder container should exist.
#[allow(dead_code)]
pub fn create_default_user_interaction() -> std::sync::Arc<MockUserInteraction> {
    std::sync::Arc::new(MockUserInteractionScenarios::never_called())
}

impl MockUserInteractionScenarios {
    /// User interaction that always says "yes" to prompts.
    #[allow(dead_code)]
    pub fn always_yes() -> MockUserInteraction {
        let mut mock = MockUserInteraction::new();
        mock.expect_prompt_yes_no()
            .times(..)
            .returning(|_| Ok(true));
        mock
    }

    /// User interaction that always says "no" to prompts.
    #[allow(dead_code)]
    pub fn always_no() -> MockUserInteraction {
        let mut mock = MockUserInteraction::new();
        mock.expect_prompt_yes_no()
            .times(..)
            .returning(|_| Ok(false));
        mock
    }

    /// User interaction that is never called (for tests where builder exists).
    pub fn never_called() -> MockUserInteraction {
        let mut mock = MockUserInteraction::new();
        mock.expect_prompt_yes_no().times(0);
        mock
    }

    /// User interaction that fails with an error.
    #[allow(dead_code)]
    pub fn error() -> MockUserInteraction {
        let mut mock = MockUserInteraction::new();
        mock.expect_prompt_yes_no()
            .times(..)
            .returning(|_| Err(anyhow::anyhow!("Failed to read user input")));
        mock
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
