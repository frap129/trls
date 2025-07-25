//! Common utilities and traits shared across trellis modules.

/// Message constants for consistent user output formatting
mod messages {
    /// Error message prefix
    pub const ERROR_PREFIX: &str = "====> ERROR: ";

    /// Warning message prefix
    pub const WARNING_PREFIX: &str = "====> WARNING: ";

    /// Info message prefix
    pub const INFO_PREFIX: &str = "====> ";
}

/// Trait providing consistent messaging functionality across all trellis components.
///
/// This trait standardizes the output format for information, warning, and error messages,
/// ensuring consistent user experience across the application.
pub trait TrellisMessaging {
    /// Displays an informational message with the standard trellis prefix.
    fn msg(&self, message: &str) {
        println!("{}{message}", messages::INFO_PREFIX);
    }

    /// Displays a warning message with the standard trellis warning prefix.
    fn warning(&self, message: &str) {
        eprintln!("{}{message}", messages::WARNING_PREFIX);
    }

    /// Displays an error message with the standard trellis error prefix.
    fn error(&self, message: &str) {
        eprintln!("{}{message}", messages::ERROR_PREFIX);
    }

    /// Displays a prompt message without a newline for user input
    fn prompt(&self, message: &str) {
        eprint!("{}{message}", messages::INFO_PREFIX);
        use std::io::{self, Write};
        let _ = io::stderr().flush();
    }
}

/// Simple messaging utility for use in main function and other contexts
/// where a full struct with TrellisMessaging is not available.
pub struct TrellisMessager;

impl TrellisMessaging for TrellisMessager {}

impl Default for TrellisMessager {
    fn default() -> Self {
        Self::new()
    }
}

impl TrellisMessager {
    pub fn new() -> Self {
        Self
    }
}
