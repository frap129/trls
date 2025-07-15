use std::process::Command;
use anyhow::{anyhow, Context, Result};

use crate::config::TrellisConfig;
use super::{
    common::{TrellisMessaging, PodmanContext},
    constants::containers,
};

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
    cmd: Command,
}

impl PodmanRunCommandBuilder {
    pub fn new() -> Self {
        let mut cmd = Command::new("podman");
        cmd.arg("run");
        Self { cmd }
    }

    pub fn network_host(mut self) -> Self {
        self.cmd.args(["--net", "host"]);
        self
    }

    pub fn add_capability(mut self, cap: ContainerCapability) -> Self {
        self.cmd.args(["--cap-add", cap.as_str()]);
        self
    }

    pub fn remove_on_exit(mut self) -> Self {
        self.cmd.arg("--rm");
        self
    }

    pub fn interactive(mut self) -> Self {
        self.cmd.arg("-it");
        self
    }

    pub fn image(mut self, image: &str) -> Self {
        self.cmd.arg(image);
        self
    }

    pub fn args(mut self, args: &[String]) -> Self {
        self.cmd.args(args);
        self
    }

    pub fn execute(mut self) -> Result<()> {
        let status = self.cmd
            .status()
            .context("Failed to execute podman run command")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("Podman run failed with exit code: {:?}", status.code()))
        }
    }
}

/// Handles container execution operations.
pub struct ContainerRunner {
}

impl TrellisMessaging for ContainerRunner {}

impl ContainerRunner {
    pub fn new(_config: &TrellisConfig) -> Self {
        Self { }
    }

    /// Runs a container with the specified tag and arguments.
    pub fn run_container(&self, container_tag: &str, args: &[String]) -> Result<()> {
        self.validate_container_exists(container_tag)?;

        PodmanRunCommandBuilder::new()
            .network_host()
            .add_capability(ContainerCapability::All)
            .remove_on_exit()
            .interactive()
            .image(&format!("{}{container_tag}", containers::LOCALHOST_PREFIX))
            .args(args)
            .execute()
    }

    /// Runs bootc upgrade with proper error handling.
    pub fn run_bootc_upgrade(&self) -> Result<()> {
        self.msg("Running bootc upgrade...");
        
        // Check if bootc is available
        self.validate_bootc_available()?;

        let output = Command::new("bootc")
            .arg("upgrade")
            .output()
            .context("Failed to execute bootc upgrade")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("bootc upgrade failed: {stderr}"));
        }

        self.msg("Update completed successfully");
        Ok(())
    }

    /// Validates that the specified container image exists.
    fn validate_container_exists(&self, container_tag: &str) -> Result<()> {
        let full_tag = format!("{}{container_tag}", containers::LOCALHOST_PREFIX);
        let output = Command::new("podman")
            .args(["image", "exists", &full_tag])
            .output()
            .podman_context("image exists check")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Container image not found: {full_tag}. Run 'trls build' first."
            ));
        }

        Ok(())
    }

    /// Validates that bootc is available and working.
    fn validate_bootc_available(&self) -> Result<()> {
        let output = Command::new("bootc")
            .arg("--version")
            .output();

        match output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(_) => Err(anyhow!("bootc is available but not responding correctly")),
            Err(_) => {
                // Try to provide helpful guidance
                if let Ok(_) = which::which("bootc") {
                    Err(anyhow!("bootc found but not executable. Check permissions."))
                } else {
                    Err(anyhow!(
                        "bootc not found. Please install bootc to use the update command."
                    ))
                }
            }
        }
    }

}
