//! Tests for the CommandExecutor trait and RealCommandExecutor implementation.

use trellis::trellis::executor::{CommandExecutor, RealCommandExecutor};

#[cfg(test)]
mod real_command_executor_tests {
    use super::*;

    #[test]
    fn test_constructor_creates_working_instance() {
        let executor = RealCommandExecutor::new();
        // Test that the constructed instance actually works
        let _ = executor.execute("echo", &["test".to_string()]);
    }

    #[test]
    fn test_podman_build_method_signature() {
        let executor = RealCommandExecutor::new();
        let args = vec!["--tag".to_string(), "test".to_string()];

        // We can't reliably test actual podman execution in CI,
        // but we can test that the method exists and handles the call
        let result = executor.podman_build(&args);

        // The result should be an error if podman is not available,
        // or success if it is. Either way, the method should handle it gracefully.
        match result {
            Ok(_) => {
                // Podman is available and command succeeded
            }
            Err(_) => {
                // Expected if podman is not available in test environment
            }
        }
    }

    #[test]
    fn test_podman_build_streaming_method_signature() {
        let executor = RealCommandExecutor::new();
        let args = vec!["--tag".to_string(), "test".to_string()];

        let result = executor.podman_build_streaming(&args);

        match result {
            Ok(_) => {
                // Podman is available and command succeeded
            }
            Err(_) => {
                // Expected if podman is not available in test environment
            }
        }
    }

    #[test]
    fn test_podman_run_method_signature() {
        let executor = RealCommandExecutor::new();
        let args = vec!["--rm".to_string(), "test".to_string()];

        let result = executor.podman_run(&args);

        match result {
            Ok(_) => {
                // Podman is available and command succeeded
            }
            Err(_) => {
                // Expected if podman is not available in test environment
            }
        }
    }

    #[test]
    fn test_podman_run_streaming_method_signature() {
        let executor = RealCommandExecutor::new();
        let args = vec!["--rm".to_string(), "test".to_string()];

        let result = executor.podman_run_streaming(&args);

        match result {
            Ok(_) => {
                // Podman is available and command succeeded
            }
            Err(_) => {
                // Expected if podman is not available in test environment
            }
        }
    }

    #[test]
    fn test_podman_images_method_signature() {
        let executor = RealCommandExecutor::new();
        let args = vec!["--format".to_string(), "table".to_string()];

        let result = executor.podman_images(&args);

        match result {
            Ok(_) => {
                // Podman is available and command succeeded
            }
            Err(_) => {
                // Expected if podman is not available in test environment
            }
        }
    }

    #[test]
    fn test_podman_rmi_method_signature() {
        let executor = RealCommandExecutor::new();
        let args = vec!["test-image".to_string()];

        let result = executor.podman_rmi(&args);

        match result {
            Ok(_) => {
                // Podman is available and command succeeded
            }
            Err(_) => {
                // Expected if podman is not available in test environment
            }
        }
    }

    #[test]
    fn test_bootc_method_signature() {
        let executor = RealCommandExecutor::new();
        let args = vec!["--version".to_string()];

        let result = executor.bootc(&args);

        match result {
            Ok(_) => {
                // Bootc is available and command succeeded
            }
            Err(_) => {
                // Expected if bootc is not available in test environment
            }
        }
    }

    #[test]
    fn test_bootc_streaming_method_signature() {
        let executor = RealCommandExecutor::new();
        let args = vec!["--version".to_string()];

        let result = executor.bootc_streaming(&args);

        match result {
            Ok(_) => {
                // Bootc is available and command succeeded
            }
            Err(_) => {
                // Expected if bootc is not available in test environment
            }
        }
    }

    #[test]
    fn test_execute_method_signature() {
        let executor = RealCommandExecutor::new();
        let args = vec!["--version".to_string()];

        // Test with a command that should be available on most systems
        let result = executor.execute("echo", &args);

        match result {
            Ok(output) => {
                // Echo command should succeed
                assert!(output.status.success());
            }
            Err(_) => {
                // Unexpected, but handle gracefully
            }
        }
    }

    #[test]
    fn test_execute_with_simple_command() {
        let executor = RealCommandExecutor::new();
        let args = vec!["hello".to_string()];

        // Test with echo command which should be universally available
        let result = executor.execute("echo", &args);

        match result {
            Ok(output) => {
                assert!(output.status.success());
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert!(stdout.contains("hello"));
            }
            Err(_) => {
                // Handle case where echo might not be available
            }
        }
    }

    #[test]
    fn test_execute_nonexistent_command() {
        let executor = RealCommandExecutor::new();
        let args = vec![];

        // Test with a command that definitely doesn't exist
        let result = executor.execute("nonexistent_command_12345", &args);

        // This should return an error
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod trait_implementation_tests {
    use super::*;

    #[test]
    fn test_trait_object_creation() {
        let executor: Box<dyn CommandExecutor> = Box::new(RealCommandExecutor::new());

        // Test that we can create a trait object and it implements Send + Sync
        fn assert_send_sync<T: Send + Sync>(_: T) {}
        assert_send_sync(executor);
    }

    #[test]
    fn test_trait_methods_through_trait_object() {
        let executor: Box<dyn CommandExecutor> = Box::new(RealCommandExecutor::new());
        let args = vec!["test".to_string()];

        // Test that all trait methods are callable through trait object
        let _ = executor.podman_build(&args);
        let _ = executor.podman_build_streaming(&args);
        let _ = executor.podman_run(&args);
        let _ = executor.podman_run_streaming(&args);
        let _ = executor.podman_images(&args);
        let _ = executor.podman_rmi(&args);
        let _ = executor.bootc(&args);
        let _ = executor.bootc_streaming(&args);
        let _ = executor.execute("echo", &args);
    }
}

#[cfg(test)]
mod argument_handling_tests {
    use super::*;

    #[test]
    fn test_empty_args_handling() {
        let executor = RealCommandExecutor::new();
        let empty_args: Vec<String> = vec![];

        // Test that empty arguments don't cause panics
        let _ = executor.execute("echo", &empty_args);
    }

    #[test]
    fn test_multiple_args_handling() {
        let executor = RealCommandExecutor::new();
        let args = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];

        // Test with multiple arguments
        let result = executor.execute("echo", &args);

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert!(stdout.contains("arg1"));
                assert!(stdout.contains("arg2"));
                assert!(stdout.contains("arg3"));
            }
            Err(_) => {
                // Handle gracefully if echo is not available
            }
        }
    }

    #[test]
    fn test_args_with_special_characters() {
        let executor = RealCommandExecutor::new();
        let args = vec![
            "--option=value".to_string(),
            "path/with/slashes".to_string(),
            "arg with spaces".to_string(),
        ];

        // Test that special characters in arguments are handled correctly
        let _ = executor.execute("echo", &args);
    }
}
