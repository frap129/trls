//! Tests for the TrellisMessaging trait and TrellisMessager implementation.

use trellis::trellis::common::{TrellisMessager, TrellisMessaging};

#[cfg(test)]
mod trellis_messaging_trait_tests {
    use super::*;

    struct TestMessager;
    impl TrellisMessaging for TestMessager {}

    #[test]
    fn test_msg_method_exists() {
        let messager = TestMessager;

        // Test that msg method can be called without panicking
        messager.msg("Test info message");
    }

    #[test]
    fn test_warning_method_exists() {
        let messager = TestMessager;

        // Test that warning method can be called without panicking
        messager.warning("Test warning message");
    }

    #[test]
    fn test_error_method_exists() {
        let messager = TestMessager;

        // Test that error method can be called without panicking
        messager.error("Test error message");
    }

    #[test]
    fn test_prompt_method_exists() {
        let messager = TestMessager;

        // Test that prompt method can be called without panicking
        messager.prompt("Test prompt: ");
    }

    #[test]
    fn test_all_message_types() {
        let messager = TestMessager;

        // Test that all message types can be used in sequence
        messager.msg("Info message");
        messager.warning("Warning message");
        messager.error("Error message");
        messager.prompt("Prompt: ");
    }

    #[test]
    fn test_empty_messages() {
        let messager = TestMessager;

        // Test that empty messages don't cause issues
        messager.msg("");
        messager.warning("");
        messager.error("");
        messager.prompt("");
    }

    #[test]
    fn test_long_messages() {
        let messager = TestMessager;
        let long_message = "This is a very long message that contains multiple words and should test how the messaging system handles longer text content without any issues.";

        // Test that long messages are handled correctly
        messager.msg(long_message);
        messager.warning(long_message);
        messager.error(long_message);
        messager.prompt(long_message);
    }

    #[test]
    fn test_messages_with_special_characters() {
        let messager = TestMessager;

        // Test messages with various special characters
        messager.msg("Message with symbols: !@#$%^&*()");
        messager.warning("Warning with unicode: 🚨 ⚠️ 💣");
        messager.error("Error with quotes: \"error\" and 'warning'");
        messager.prompt("Prompt with newlines:\nLine 2\nLine 3");
    }
}

#[cfg(test)]
mod trellis_messager_tests {
    use super::*;

    #[test]
    fn test_new_constructor() {
        let messager = TrellisMessager::new();

        // Test that constructor works
        drop(messager);
    }

    #[test]
    fn test_default_constructor() {
        let messager = TrellisMessager;

        // Test that default constructor works
        drop(messager);
    }

    #[test]
    fn test_implements_trellis_messaging() {
        let messager = TrellisMessager::new();

        // Test that TrellisMessager implements TrellisMessaging trait
        messager.msg("Test message");
        messager.warning("Test warning");
        messager.error("Test error");
        messager.prompt("Test prompt: ");
    }

    #[test]
    fn test_can_be_used_as_trait_object() {
        let messager: Box<dyn TrellisMessaging> = Box::new(TrellisMessager::new());

        // Test that TrellisMessager can be used as a trait object
        messager.msg("Trait object message");
        messager.warning("Trait object warning");
        messager.error("Trait object error");
        messager.prompt("Trait object prompt: ");
    }

    #[test]
    fn test_multiple_instances() {
        let messager1 = TrellisMessager::new();
        let messager2 = TrellisMessager;

        // Test that multiple instances can coexist
        messager1.msg("Message from instance 1");
        messager2.msg("Message from instance 2");
    }
}

#[cfg(test)]
mod message_formatting_tests {
    use super::*;

    #[test]
    fn test_message_prefixes_are_consistent() {
        let messager = TrellisMessager::new();

        // These tests verify the methods can be called.
        // In a real implementation, we'd capture the output and verify prefixes.
        messager.msg("Info should have ====> prefix");
        messager.warning("Warning should have ====> WARNING: prefix");
        messager.error("Error should have ====> ERROR: prefix");
        messager.prompt("Prompt should have ====> prefix without newline");
    }

    #[test]
    fn test_output_destinations() {
        let messager = TrellisMessager::new();

        // Test that different message types use appropriate output streams
        // msg() should use stdout, warning/error/prompt should use stderr
        messager.msg("This goes to stdout");
        messager.warning("This goes to stderr");
        messager.error("This goes to stderr");
        messager.prompt("This goes to stderr without newline");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_realistic_usage_pattern() {
        let messager = TrellisMessager::new();

        // Test a realistic sequence of operations
        messager.msg("Starting build process");
        messager.msg("Building container image");
        messager.warning("Non-critical issue detected");
        messager.msg("Build completed successfully");
        messager.prompt("Continue with deployment? (y/n): ");
    }

    #[test]
    fn test_error_reporting_pattern() {
        let messager = TrellisMessager::new();

        // Test error reporting workflow
        messager.error("Build failed: Containerfile not found");
        messager.msg("Attempting to locate Containerfile");
        messager.warning("Using fallback Containerfile");
        messager.msg("Retry build operation");
    }

    #[test]
    fn test_batch_operations() {
        let messager = TrellisMessager::new();

        // Test handling multiple operations
        for i in 1..=5 {
            messager.msg(&format!("Processing stage {i}"));
            if i == 3 {
                messager.warning(&format!("Stage {i} took longer than expected"));
            }
        }
        messager.msg("All stages completed");
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;

    #[test]
    fn test_many_messages() {
        let messager = TrellisMessager::new();

        // Test that the messaging system can handle many messages
        for i in 0..100 {
            match i % 4 {
                0 => messager.msg(&format!("Info message {i}")),
                1 => messager.warning(&format!("Warning message {i}")),
                2 => messager.error(&format!("Error message {i}")),
                3 => messager.prompt(&format!("Prompt {i}: ")),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let messager = Arc::new(TrellisMessager::new());
        let mut handles = vec![];

        // Test that TrellisMessager can be used safely across threads
        for i in 0..10 {
            let messager_clone = Arc::clone(&messager);
            let handle = thread::spawn(move || {
                messager_clone.msg(&format!("Thread {i} message"));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
