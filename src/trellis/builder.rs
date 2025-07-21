use anyhow::{anyhow, Context, Result};
use std::{env, fs, sync::Arc};

use super::{
    common::TrellisMessaging, constants::env_vars, discovery::ContainerfileDiscovery,
    executor::CommandExecutor,
};
use crate::config::TrellisConfig;

/// Type of container build operation.
#[derive(Debug, Clone, Copy)]
pub enum BuildType {
    Builder,
    Rootfs,
}

/// Manages environment variables with automatic cleanup.
pub struct ScopedEnvVar {
    key: String,
    original_value: Option<String>,
}

impl ScopedEnvVar {
    pub fn new(key: &str, value: &str) -> Self {
        let original_value = env::var(key).ok();
        env::set_var(key, value);
        Self {
            key: key.to_string(),
            original_value,
        }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        match &self.original_value {
            Some(value) => env::set_var(&self.key, value),
            None => env::remove_var(&self.key),
        }
    }
}

/// Builder for constructing podman commands with type safety.
pub struct PodmanCommandBuilder {
    args: Vec<String>,
}

impl Default for PodmanCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PodmanCommandBuilder {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    /// Creates a new build command with standard capabilities
    pub fn new_build_command() -> Self {
        Self::new()
            .build_subcommand()
            .network_host()
            .add_capability("sys_admin")
            .add_capability("mknod")
            .squash()
    }

    pub fn build_subcommand(self) -> Self {
        // build subcommand is implicit in executor.podman_build
        self
    }

    pub fn no_cache(mut self, no_cache: bool) -> Self {
        if no_cache {
            self.args.push("--no-cache".to_string());
        }
        self
    }

    pub fn network_host(mut self) -> Self {
        self.args.extend(["--net".to_string(), "host".to_string()]);
        self
    }

    pub fn add_capability(mut self, cap: &str) -> Self {
        self.args.extend(["--cap-add".to_string(), cap.to_string()]);
        self
    }

    pub fn squash(mut self) -> Self {
        self.args.push("--squash".to_string());
        self
    }

    pub fn containerfile<P: AsRef<std::path::Path>>(mut self, path: P) -> Self {
        self.args.extend([
            "-f".to_string(),
            path.as_ref().to_string_lossy().to_string(),
        ]);
        self
    }

    pub fn build_arg(mut self, key: &str, value: &str) -> Self {
        self.args
            .extend(["--build-arg".to_string(), format!("{key}={value}")]);
        self
    }

    pub fn target(mut self, target: &str) -> Self {
        self.args
            .extend(["--target".to_string(), target.to_string()]);
        self
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.args.extend(["-t".to_string(), tag.to_string()]);
        self
    }

    pub fn volume(mut self, mount: &str) -> Self {
        self.args.extend(["-v".to_string(), mount.to_string()]);
        self
    }

    pub fn build_context(mut self, context: &str) -> Self {
        self.args
            .extend(["--build-context".to_string(), context.to_string()]);
        self
    }

    /// Returns the collected arguments for execution via CommandExecutor.
    pub fn build_args(self) -> Vec<String> {
        self.args
    }
}

/// Handles container building operations.
pub struct ContainerBuilder<'a> {
    config: &'a TrellisConfig,
    discovery: ContainerfileDiscovery<'a>,
    executor: Arc<dyn CommandExecutor>,
}

impl<'a> TrellisMessaging for ContainerBuilder<'a> {}

impl<'a> ContainerBuilder<'a> {
    pub fn new(config: &'a TrellisConfig, executor: Arc<dyn CommandExecutor>) -> Self {
        Self {
            config,
            discovery: ContainerfileDiscovery::new(config),
            executor,
        }
    }

    /// Determines the base image for a given stage in the build process.
    /// This method is primarily for testing the base image selection logic.
    pub fn determine_base_image(
        &self,
        _stage_index: usize,
        build_type: BuildType,
        last_stage: &str,
    ) -> String {
        if last_stage.is_empty() {
            match build_type {
                BuildType::Rootfs => self.config.rootfs_base.clone(),
                BuildType::Builder => "scratch".to_string(),
            }
        } else {
            format!("localhost/{last_stage}")
        }
    }

    /// Builds a multi-stage container with improved error handling and resource management.
    pub fn build_multistage_container(
        &self,
        tmp_name: &str,
        final_tag: &str,
        build_stages: &[String],
        build_type: BuildType,
    ) -> Result<()> {
        // Validate all containerfiles exist upfront
        self.discovery.validate_stages(build_stages)?;

        let mut last_stage = String::new();
        let _scoped_env = if !self.config.podman_build_cache {
            Some(ScopedEnvVar::new(env_vars::BUILDAH_LAYERS, "false"))
        } else {
            None
        };

        for (i, build_stage) in build_stages.iter().enumerate() {
            let (group, stage) = ContainerfileDiscovery::parse_stage_name(build_stage);

            let tag = if i == build_stages.len() - 1 {
                final_tag.to_string()
            } else if stage != group {
                format!("trellis-{tmp_name}-{group}-{stage}")
            } else {
                format!("trellis-{tmp_name}-{stage}")
            };

            // For "group:stage" syntax, look for Containerfile.group containing stage target
            let containerfile_path = if stage != group {
                // group:stage syntax - look for file named after group
                self.discovery.find_containerfile(&group)?
            } else {
                // simple stage name - look for file named after stage
                self.discovery.find_containerfile(&stage)?
            };

            self.msg(&format!(
                "Building stage {}/{}: {} -> {}",
                i + 1,
                build_stages.len(),
                build_stage,
                tag
            ));

            // For the first stage, use rootfs_base as BASE_IMAGE; for subsequent stages, use the previous stage
            let base_image = self.determine_base_image(i, build_type, &last_stage);

            let mut builder = PodmanCommandBuilder::new_build_command()
                .containerfile(&containerfile_path)
                .build_arg("BASE_IMAGE", &base_image)
                .target(&stage)
                .tag(&tag)
                .no_cache(!self.config.podman_build_cache);

            // Add rootfs-specific configuration
            if matches!(build_type, BuildType::Rootfs) {
                builder = self.add_rootfs_config(builder)?;
            }

            // Execute build using injected executor
            let build_args = builder.build_args();
            let success = if self.config.quiet {
                // Use regular execution to capture output when quiet
                let output = self
                    .executor
                    .podman_build(&build_args)
                    .with_context(|| format!("Failed to build stage: {build_stage}"))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow!("Podman build failed with exit code: {:?}. Error: {}. Check podman logs for details. Ensure sufficient disk space and proper permissions.", output.status.code(), stderr));
                }
                output.status.success()
            } else {
                // Use streaming execution to show live output
                let status = self
                    .executor
                    .podman_build_streaming(&build_args)
                    .with_context(|| format!("Failed to build stage: {build_stage}"))?;

                if !status.success() {
                    return Err(anyhow!("Podman build failed with exit code: {:?}. Check podman logs for details. Ensure sufficient disk space and proper permissions.", status.code()));
                }
                status.success()
            };

            if !success {
                return Err(anyhow!("Build process failed unexpectedly"));
            }

            last_stage = tag;
        }

        Ok(())
    }

    /// Adds rootfs-specific configuration to the podman command builder.
    fn add_rootfs_config(&self, mut builder: PodmanCommandBuilder) -> Result<PodmanCommandBuilder> {
        // Add build contexts
        for context in &self.config.extra_contexts {
            builder = builder.build_context(context);
        }

        // Add cache directories
        builder = self.add_cache_mount(
            builder,
            &self.config.pacman_cache,
            "pacman",
            "/var/cache/pacman/pkg",
        )?;
        builder = self.add_cache_mount(
            builder,
            &self.config.aur_cache,
            "AUR",
            "/var/cache/trellis/aur",
        )?;

        // Add hooks directory
        if let Some(hooks_dir) = &self.config.hooks_dir {
            let hooks_path = hooks_dir.display().to_string();
            builder = builder
                .volume(&format!("{hooks_path}:{hooks_path}"))
                .build_arg("HOOKS_DIR", &hooks_path);
        }

        // Add extra mounts
        for mount in &self.config.extra_mounts {
            let mount_path = mount.display().to_string();
            builder = builder.volume(&format!("{mount_path}:{mount_path}"));
        }

        Ok(builder)
    }

    /// Adds cache mount configuration with proper validation and error categorization.
    fn add_cache_mount(
        &self,
        mut builder: PodmanCommandBuilder,
        cache_path: &Option<std::path::PathBuf>,
        cache_name: &str,
        container_path: &str,
    ) -> Result<PodmanCommandBuilder> {
        if let Some(cache_dir) = cache_path {
            // Try to create the cache directory
            if let Err(e) = fs::create_dir_all(cache_dir) {
                let error_msg =
                    format!("Failed to create {cache_name} cache directory: {cache_dir:?} - {e}");
                self.warning(&error_msg);

                // Categorize the error for better user feedback
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    return Err(anyhow!(
                        "Permission denied creating {cache_name} cache directory. Try running with elevated privileges or choose a different cache location."
                    ));
                } else {
                    return Err(anyhow!("{error_msg}"));
                }
            }

            // Verify the directory is accessible and writable
            match cache_dir.metadata() {
                Ok(metadata) => {
                    if metadata.permissions().readonly() {
                        let error_msg =
                            format!("{cache_name} cache directory is read-only: {cache_dir:?}");
                        self.warning(&error_msg);
                        return Err(anyhow!("{error_msg}"));
                    }

                    builder = builder.volume(&format!("{}:{container_path}", cache_dir.display()));
                    self.msg(&format!(
                        "Using {cache_name} cache: {}",
                        cache_dir.display()
                    ));
                }
                Err(e) => {
                    let error_msg =
                        format!("Cannot access {cache_name} cache directory: {cache_dir:?} - {e}");
                    self.warning(&error_msg);
                    return Err(anyhow!("{error_msg}"));
                }
            }
        }
        Ok(builder)
    }
}
