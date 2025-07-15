//! Common utilities and traits shared across trellis modules.

use anyhow::Context;

/// Trait providing consistent messaging functionality across all trellis components.
/// 
/// This trait standardizes the output format for information, warning, and error messages,
/// ensuring consistent user experience across the application.
pub trait TrellisMessaging {
    /// Displays an informational message with the standard trellis prefix.
    fn msg(&self, message: &str) {
        println!("====> {message}");
    }
    
    /// Displays a warning message with the standard trellis warning prefix.
    fn warning(&self, message: &str) {
        eprintln!("====> WARNING: {message}");
    }
    
    /// Displays an error message with the standard trellis error prefix.
    fn error(&self, message: &str) {
        eprintln!("====> ERROR: {message}");
    }
}

/// Simple messaging utility for use in main function and other contexts
/// where a full struct with TrellisMessaging is not available.
pub struct TrellisMessager;

impl TrellisMessaging for TrellisMessager {}

impl TrellisMessager {
    pub fn new() -> Self {
        Self
    }
}

/// Extension trait for adding podman-specific error context to Results.
/// 
/// This trait provides convenient methods for adding consistent error context
/// to podman command operations throughout the codebase.
pub trait PodmanContext<T> {
    /// Adds standardized context for podman command failures.
    fn podman_context(self, operation: &str) -> anyhow::Result<T>;
}

impl<T, E> PodmanContext<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn podman_context(self, operation: &str) -> anyhow::Result<T> {
        self.with_context(|| format!("Failed to execute podman {operation}"))
    }
}