use anyhow::{anyhow, Context, Result};
use std::sync::Arc;

use super::{common::TrellisMessaging, constants::containers, executor::CommandExecutor};
use crate::config::TrellisConfig;

/// Container capabilities enum for type safety.
/// These capabilities are granted to containers to enable specific operations.
#[derive(Debug, Clone, Copy)]
pub enum ContainerCapability {
    /// SYS_ADMIN: Enables system administration operations like mount, namespace management
    SysAdmin,
    /// DAC_OVERRIDE: Allows bypassing file permission checks (needed for package management)
    DacOverride,
    /// CHOWN: Required for changing file ownership (used by package managers and scripts)
    Chown,
    /// FOWNER: Allows changing file permissions without ownership (needed for some install scripts)
    Fowner,
    /// SETUID: Allows setting the UID of processes (used by package scripts and su/sudo)
    Setuid,
    /// SETGID: Allows setting the GID of processes (used for group-based operations)
    Setgid,
    /// SYS_PTRACE: Required for tracing and debugging operations (some package managers need this)
    SysPtrace,
}

impl ContainerCapability {
    fn as_str(self) -> &'static str {
        match self {
            ContainerCapability::SysAdmin => "SYS_ADMIN",
            ContainerCapability::DacOverride => "DAC_OVERRIDE",
            ContainerCapability::Chown => "CHOWN",
            ContainerCapability::Fowner => "FOWNER",
            ContainerCapability::Setuid => "SETUID",
            ContainerCapability::Setgid => "SETGID",
            ContainerCapability::SysPtrace => "SYS_PTRACE",
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

    pub fn name(mut self, name: &str) -> Self {
        self.args.extend(["--name".to_string(), name.to_string()]);
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
            // Use host network namespace (--net host) to enable network access
            // This allows the container to resolve DNS, access package mirrors, and perform
            // any network operations (e.g., git clones, package downloads) required by
            // the commands executed inside the container.
            .network_host()
            // Minimal capabilities required for interactive container use:
            // - SYS_ADMIN: needed for system-level operations and namespace management
            // - DAC_OVERRIDE: allows bypassing file permission checks
            // - CHOWN: required when changing file ownership
            // - FOWNER: allows changing file permissions without being the owner
            // - SETUID/SETGID: needed for user switching and privilege-related operations
            // - SYS_PTRACE: allows tracing and debugging operations
            .add_capability(ContainerCapability::SysAdmin)
            .add_capability(ContainerCapability::DacOverride)
            .add_capability(ContainerCapability::Chown)
            .add_capability(ContainerCapability::Fowner)
            .add_capability(ContainerCapability::Setuid)
            .add_capability(ContainerCapability::Setgid)
            .add_capability(ContainerCapability::SysPtrace)
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

    /// Performs a quick update by running topgrade in the existing rootfs container
    /// and committing the changes back to the same tag.
    pub fn quick_update_rootfs(&self) -> Result<()> {
        let rootfs_tag = &self.config.rootfs_tag;

        // Validate that the rootfs container exists
        self.validate_container_exists(rootfs_tag)?;

        // Check if topgrade is available in the container
        self.validate_topgrade_in_container(rootfs_tag)?;

        self.msg("Starting quick update process...");

        // Generate a unique container name for the update process
        let duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
        let timestamp = duration.as_secs() * 1_000_000_000 + duration.subsec_nanos() as u64;
        let container_name = format!("trellis-quick-update-{}", timestamp);

        // Step 1: Run topgrade in a new container
        self.run_topgrade_in_container(rootfs_tag, &container_name)?;

        // Step 2: Attempt to commit the updated container, but ensure cleanup ALWAYS runs
        let commit_result = self.commit_container_updates(&container_name, rootfs_tag);

        // Step 3: ALWAYS clean up the temporary container, regardless of commit result
        let _cleanup_result = self.cleanup_temporary_container(&container_name);

        // Step 4: Return the commit result (must happen after cleanup)
        commit_result?;

        self.msg("Quick update completed successfully");

        Ok(())
    }

    /// Validates that topgrade is available in the specified container.
    fn validate_topgrade_in_container(&self, container_tag: &str) -> Result<()> {
        self.msg("Checking for topgrade availability in container...");

        if !self
            .executor
            .check_command_in_container(container_tag, "topgrade")?
        {
            return Err(anyhow!(
                "topgrade is not available in the container: {}. \
                 Please rebuild your rootfs container with topgrade installed, \
                 or use the regular 'update' command instead.",
                container_tag
            ));
        }

        Ok(())
    }

    /// Runs topgrade inside a container to update packages.
    fn run_topgrade_in_container(&self, rootfs_tag: &str, container_name: &str) -> Result<()> {
        self.msg("Running topgrade to update packages...");

        let run_args = PodmanRunCommandBuilder::new()
            // Use host network namespace (--net host) to enable full network access
            // This is critical for topgrade functionality:
            // - Topgrade downloads package updates from package mirrors
            // - Container must resolve DNS to reach mirror servers
            // - Network access allows fetching updates for multiple package managers
            //   (pacman, yay, cargo, pip, etc.)
            // - Without host network, the container would be isolated and unable to
            //   access internet resources needed for package updates
            .network_host()
            // Minimal capabilities required for package management via topgrade:
            // - SYS_ADMIN: needed for system-level operations and namespace management
            // - DAC_OVERRIDE: allows bypassing file permission checks during package installation
            // - CHOWN: required when packages change file ownership (common in post-install scripts)
            // - FOWNER: allows changing file permissions without being the owner
            // - SETUID/SETGID: package scripts may use su/sudo or need to set UIDs
            // - SYS_PTRACE: some package managers trace processes during installation
            .add_capability(ContainerCapability::SysAdmin)
            .add_capability(ContainerCapability::DacOverride)
            .add_capability(ContainerCapability::Chown)
            .add_capability(ContainerCapability::Fowner)
            .add_capability(ContainerCapability::Setuid)
            .add_capability(ContainerCapability::Setgid)
            .add_capability(ContainerCapability::SysPtrace)
            .name(container_name)
            .image(&format!("{}{}", containers::LOCALHOST_PREFIX, rootfs_tag))
            .args(&["topgrade".to_string(), "-y".to_string()])
            .run_args();

        if self.config.quiet {
            let output = self
                .executor
                .podman_run(&run_args)
                .context("Failed to run topgrade in container")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!(
                    "topgrade failed with exit code: {:?}. Error: {}",
                    output.status.code(),
                    stderr
                ));
            }
        } else {
            let status = self
                .executor
                .podman_run_streaming(&run_args)
                .context("Failed to run topgrade in container")?;

            if !status.success() {
                return Err(anyhow!(
                    "topgrade failed with exit code: {:?}",
                    status.code()
                ));
            }
        }

        Ok(())
    }

    /// Commits the updated container back to the original rootfs tag.
    fn commit_container_updates(&self, container_name: &str, rootfs_tag: &str) -> Result<()> {
        self.msg("Committing container updates...");

        let commit_args = vec![
            container_name.to_string(),
            format!("localhost/{}", rootfs_tag),
        ];

        let output = self
            .executor
            .podman_commit(&commit_args)
            .context("Failed to commit container updates")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to commit container: {}", stderr));
        }

        Ok(())
    }

    /// Removes the temporary container used for the update process.
    fn cleanup_temporary_container(&self, container_name: &str) -> Result<()> {
        self.msg("Cleaning up temporary container...");

        let args = vec!["rm".to_string(), container_name.to_string()];
        let output = self
            .executor
            .execute("podman", &args)
            .context("Failed to remove temporary container")?;

        if !output.status.success() {
            self.warning(&format!(
                "Failed to clean up temporary container '{}'. You may need to remove it manually.",
                container_name
            ));
        }

        Ok(())
    }
}
