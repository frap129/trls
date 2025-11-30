//! Command execution abstraction for dependency injection.
//!
//! This module provides traits and implementations for abstracting external
//! command execution, enabling comprehensive testing through mocking.

use anyhow::Result;
use std::process::{ExitStatus, Output};

/// Trait for executing external commands.
///
/// This trait abstracts all external command execution, allowing for
/// easy mocking in tests and potential future alternative implementations.
pub trait CommandExecutor: Send + Sync {
    /// Execute a podman build command.
    fn podman_build(&self, args: &[String]) -> Result<Output>;

    /// Execute a podman build command with streaming output.
    fn podman_build_streaming(&self, args: &[String]) -> Result<ExitStatus>;

    /// Execute a podman run command.
    fn podman_run(&self, args: &[String]) -> Result<Output>;

    /// Execute a podman run command with streaming output.
    fn podman_run_streaming(&self, args: &[String]) -> Result<ExitStatus>;

    /// Execute a podman images command.
    fn podman_images(&self, args: &[String]) -> Result<Output>;

    /// Execute a podman rmi command.
    fn podman_rmi(&self, args: &[String]) -> Result<Output>;

    /// Execute a podman commit command.
    fn podman_commit(&self, args: &[String]) -> Result<Output>;

    /// Check if a command is available in a container.
    fn check_command_in_container(&self, container_tag: &str, command: &str) -> Result<bool>;

    /// Execute a bootc command.
    fn bootc(&self, args: &[String]) -> Result<Output>;

    /// Execute a bootc command with streaming output.
    fn bootc_streaming(&self, args: &[String]) -> Result<ExitStatus>;

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

    fn podman_build_streaming(&self, args: &[String]) -> Result<ExitStatus> {
        let status = std::process::Command::new("podman")
            .arg("build")
            .args(args)
            .status()?;
        Ok(status)
    }

    fn podman_run(&self, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new("podman")
            .arg("run")
            .args(args)
            .output()?;
        Ok(output)
    }

    fn podman_run_streaming(&self, args: &[String]) -> Result<ExitStatus> {
        let status = std::process::Command::new("podman")
            .arg("run")
            .args(args)
            .status()?;
        Ok(status)
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

    fn podman_commit(&self, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new("podman")
            .arg("commit")
            .args(args)
            .output()?;
        Ok(output)
    }

    fn check_command_in_container(&self, container_tag: &str, command: &str) -> Result<bool> {
        // Run a test command in the container image to check if command exists
        let output = std::process::Command::new("podman")
            .arg("run")
            .arg("--rm")
            .arg(format!("localhost/{}", container_tag))
            .arg("sh")
            .arg("-c")
            .arg(format!("which {}", command))
            .output()?;
        Ok(output.status.success())
    }

    fn bootc(&self, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new("bootc")
            .args(args)
            .env("LC_ALL", "C.UTF-8")
            .output()?;
        Ok(output)
    }

    fn bootc_streaming(&self, args: &[String]) -> Result<ExitStatus> {
        let status = std::process::Command::new("bootc")
            .args(args)
            .env("LC_ALL", "C.UTF-8")
            .status()?;
        Ok(status)
    }

    fn execute(&self, command: &str, args: &[String]) -> Result<Output> {
        let output = std::process::Command::new(command).args(args).output()?;
        Ok(output)
    }
}
