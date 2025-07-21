use anyhow::{anyhow, Context, Result};
use std::sync::Arc;

use super::{common::TrellisMessaging, constants::containers, executor::CommandExecutor};
use crate::config::TrellisConfig;

/// Container capabilities enum for type safety.
#[derive(Debug, Clone, Copy)]
pub enum ContainerCapability {
    All,
}

impl ContainerCapability {
    fn as_str(self) -> &'static str {
        match self {
            ContainerCapability::All => "all",
        }
    }
}

/// Builder for constructing podman run commands.
pub struct PodmanRunCommandBuilder {
    args: Vec<String>,
}

impl Default for PodmanRunCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PodmanRunCommandBuilder {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    pub fn network_host(mut self) -> Self {
        self.args.extend(["--net".to_string(), "host".to_string()]);
        self
    }

    pub fn add_capability(mut self, cap: ContainerCapability) -> Self {
        self.args
            .extend(["--cap-add".to_string(), cap.as_str().to_string()]);
        self
    }

    pub fn remove_on_exit(mut self) -> Self {
        self.args.push("--rm".to_string());
        self
    }

    pub fn interactive(mut self) -> Self {
        self.args.push("-it".to_string());
        self
    }

    pub fn image(mut self, image: &str) -> Self {
        self.args.push(image.to_string());
        self
    }

    pub fn args(mut self, args: &[String]) -> Self {
        self.args.extend(args.iter().cloned());
        self
    }

    /// Returns the collected arguments for execution via CommandExecutor.
    pub fn run_args(self) -> Vec<String> {
        self.args
    }
}

/// Handles container execution operations.
pub struct ContainerRunner<'a> {
    config: &'a TrellisConfig,
    executor: Arc<dyn CommandExecutor>,
}

impl<'a> TrellisMessaging for ContainerRunner<'a> {}

impl<'a> ContainerRunner<'a> {
    pub fn new(config: &'a TrellisConfig, executor: Arc<dyn CommandExecutor>) -> Self {
        Self { config, executor }
    }

    /// Runs a container with the specified tag and arguments.
    pub fn run_container(&self, container_tag: &str, args: &[String]) -> Result<()> {
        self.validate_container_exists(container_tag)?;

        let run_args = PodmanRunCommandBuilder::new()
            .network_host()
            .add_capability(ContainerCapability::All)
            .remove_on_exit()
            .interactive()
            .image(&format!("{}{container_tag}", containers::LOCALHOST_PREFIX))
            .args(args)
            .run_args();

        let success = if self.config.quiet {
            // Use regular execution to capture output when quiet
            let output = self
                .executor
                .podman_run(&run_args)
                .context("Failed to execute podman run command")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!(
                    "Podman run failed with exit code: {:?}. Error: {}",
                    output.status.code(),
                    stderr
                ));
            }
            output.status.success()
        } else {
            // Use streaming execution to show live output
            let status = self
                .executor
                .podman_run_streaming(&run_args)
                .context("Failed to execute podman run command")?;

            if !status.success() {
                return Err(anyhow!(
                    "Podman run failed with exit code: {:?}",
                    status.code()
                ));
            }
            status.success()
        };

        if !success {
            return Err(anyhow!("Run process failed unexpectedly"));
        }

        Ok(())
    }

    /// Runs bootc upgrade with proper error handling.
    pub fn run_bootc_upgrade(&self) -> Result<()> {
        self.msg("Running bootc upgrade...");

        // Check if bootc is available
        self.validate_bootc_available()?;

        let args = vec!["upgrade".to_string()];
        let success = if self.config.quiet {
            // Use regular execution to capture output when quiet
            let output = self
                .executor
                .bootc(&args)
                .context("Failed to execute bootc upgrade")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("bootc upgrade failed: {stderr}"));
            }
            output.status.success()
        } else {
            // Use streaming execution to show live output
            let status = self
                .executor
                .bootc_streaming(&args)
                .context("Failed to execute bootc upgrade")?;

            if !status.success() {
                return Err(anyhow!(
                    "bootc upgrade failed with exit code: {:?}",
                    status.code()
                ));
            }
            status.success()
        };

        if !success {
            return Err(anyhow!("Bootc upgrade process failed unexpectedly"));
        }

        self.msg("Update completed successfully");
        Ok(())
    }

    /// Validates that the specified container image exists.
    fn validate_container_exists(&self, container_tag: &str) -> Result<()> {
        let full_tag = format!("{}{container_tag}", containers::LOCALHOST_PREFIX);
        let args = vec!["image".to_string(), "exists".to_string(), full_tag.clone()];
        let output = self
            .executor
            .execute("podman", &args)
            .context("Failed to check if image exists")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Container image not found: {full_tag}. Run 'trls build' first."
            ));
        }

        Ok(())
    }

    /// Validates that bootc is available and working.
    fn validate_bootc_available(&self) -> Result<()> {
        let args = vec!["--version".to_string()];
        let output = self.executor.bootc(&args);

        match output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(_) => Err(anyhow!("bootc is available but not responding correctly")),
            Err(_) => Err(anyhow!(
                "bootc is not available. Please install bootc to use the update command."
            )),
        }
    }
}
