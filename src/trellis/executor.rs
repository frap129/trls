//! Command execution abstraction for dependency injection.
//!
//! This module provides traits and implementations for abstracting external
//! command execution, enabling comprehensive testing through mocking.

use anyhow::Result;
use std::process::Output;

/// Trait for executing external commands.
///
/// This trait abstracts all external command execution, allowing for
/// easy mocking in tests and potential future alternative implementations.
pub trait CommandExecutor: Send + Sync {
    /// Execute a podman build command.
    fn podman_build(&self, args: &[String]) -> Result<Output>;

    /// Execute a podman run command.
    fn podman_run(&self, args: &[String]) -> Result<Output>;

    /// Execute a podman images command.
    fn podman_images(&self, args: &[String]) -> Result<Output>;

    /// Execute a podman rmi command.
    fn podman_rmi(&self, args: &[String]) -> Result<Output>;

    /// Execute a bootc command.
    fn bootc(&self, args: &[String]) -> Result<Output>;

    /// Execute any generic command.
    fn execute(&self, command: &str, args: &[String]) -> Result<Output>;
}

/// Real command executor for production use.
pub struct RealCommandExecutor;

impl RealCommandExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RealCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor for RealCommandExecutor {
    fn podman_build(&self, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new("podman")
            .arg("build")
            .args(args)
            .output()?;
        Ok(output)
    }

    fn podman_run(&self, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new("podman")
            .arg("run")
            .args(args)
            .output()?;
        Ok(output)
    }

    fn podman_images(&self, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new("podman")
            .arg("images")
            .args(args)
            .output()?;
        Ok(output)
    }

    fn podman_rmi(&self, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new("podman")
            .arg("rmi")
            .args(args)
            .output()?;
        Ok(output)
    }

    fn bootc(&self, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new("bootc").args(args).output()?;
        Ok(output)
    }

    fn execute(&self, command: &str, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new(command).args(args).output()?;
        Ok(output)
    }
}
