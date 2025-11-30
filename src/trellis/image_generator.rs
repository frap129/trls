//! Bootable disk image generation functionality.
//!
//! This module provides the ability to generate bootable disk images from
//! built containers, following the methodology from arch-bootc project.
//!
//! The generation flow is:
//! 1. Validate the container image exists
//! 2. Create the disk image file
//! 3. Run `bootc install` to install the container to the disk image
//! 4. Mount the installed disk image and inject trellis configuration

use anyhow::{anyhow, Context, Result};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{common::TrellisMessaging, executor::CommandExecutor};
use crate::config::{Config, TrellisConfig};

/// Resolves image tags to full format with registry and tag suffix.
///
/// This function normalizes image tags from CLI or config sources into a consistent format
/// following these rules:
///
/// - If the CLI provides an image with both registry and tag (e.g., `registry.io/image:tag`), use it as-is
/// - If the CLI provides an image with registry but no tag (e.g., `registry.io/image`), append `:latest`
/// - If the CLI provides an image with tag but no registry (e.g., `myimage:v1`), prepend `localhost/`
/// - If the CLI provides just a name (e.g., `myimage`), prepend `localhost/` and append `:latest`
/// - If no CLI image provided, use config's `rootfs_tag` with `localhost/` prefix and `:latest` suffix
///
/// # Arguments
///
/// * `config` - The Trellis configuration containing the default rootfs_tag
/// * `cli_image` - Optional image tag provided via CLI
///
/// # Examples
///
/// ```ignore
/// let config = TrellisConfig { rootfs_tag: "trellis-root".to_string(), .. };
/// assert_eq!(resolve_image_tag(&config, Some("registry.io/my-image:v1")), "registry.io/my-image:v1");
/// assert_eq!(resolve_image_tag(&config, Some("registry.io/my-image")), "registry.io/my-image:latest");
/// assert_eq!(resolve_image_tag(&config, Some("my-image:v1")), "localhost/my-image:v1");
/// assert_eq!(resolve_image_tag(&config, Some("my-image")), "localhost/my-image:latest");
/// assert_eq!(resolve_image_tag(&config, None), "localhost/trellis-root:latest");
/// ```
pub fn resolve_image_tag(config: &TrellisConfig, cli_image: Option<&str>) -> String {
    match cli_image {
        None => format!("localhost/{}:latest", config.rootfs_tag),
        Some(tag) => {
            let has_registry = tag.contains('/');
            let has_version = tag.contains(':');

            match (has_registry, has_version) {
                // e.g., "registry.io/my-image:v1" -> "registry.io/my-image:v1"
                (true, true) => tag.to_string(),
                // e.g., "registry.io/my-image" -> "registry.io/my-image:latest"
                (true, false) => format!("{}:latest", tag),
                // e.g., "my-image:v1" -> "localhost/my-image:v1"
                (false, true) => format!("localhost/{}", tag),
                // e.g., "my-image" -> "localhost/my-image:latest"
                (false, false) => format!("localhost/{}:latest", tag),
            }
        }
    }
}

/// Handles bootable disk image generation operations.
pub struct ImageGenerator<'a> {
    config: &'a TrellisConfig,
    executor: Arc<dyn CommandExecutor>,
}

impl<'a> ImageGenerator<'a> {
    /// Create a new image generator.
    pub fn new(config: &'a TrellisConfig, executor: Arc<dyn CommandExecutor>) -> Self {
        Self { config, executor }
    }

    /// Generate a bootable disk image from a container image.
    ///
    /// # Arguments
    ///
    /// * `image_tag` - The container image tag to use for generation
    /// * `output_path` - Path where the image file should be created
    /// * `filesystem` - Filesystem type for the image (e.g., "ext4")
    /// * `size_gb` - Size of the image in gigabytes
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Container image doesn't exist
    /// - Any operation in the generation process fails
    pub fn generate_bootable_image(
        &self,
        image_tag: &str,
        output_path: &Path,
        filesystem: &str,
        size_gb: u64,
    ) -> Result<()> {
        self.msg(&format!("Generating bootable image from {}", image_tag));

        // Validate image exists
        self.validate_image_exists(image_tag)?;

        // Create image file
        self.create_image_file(output_path, size_gb)?;

        // Install bootable system using the ORIGINAL container image
        self.install_bootable_system(image_tag, output_path, filesystem)?;

        // Inject trellis configuration into the INSTALLED disk image
        self.inject_configuration_to_disk(output_path)?;

        self.msg("Bootable image generated successfully");
        Ok(())
    }

    /// Validate that the specified container image exists.
    pub fn validate_image_exists(&self, image_tag: &str) -> Result<()> {
        let args = vec![
            "--filter".to_string(),
            format!("reference={}", image_tag),
            "--format".to_string(),
            "{{.Repository}}:{{.Tag}}".to_string(),
        ];

        let output = self
            .executor
            .podman_images(&args)
            .context("Failed to check if container image exists")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Podman images command failed with exit code: {:?}",
                output.status.code()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Err(anyhow!(
                "Container image '{}' not found. Build it first or use --build flag.",
                image_tag
            ));
        }

        Ok(())
    }

    /// Create the image file using fallocate.
    pub fn create_image_file(&self, output_path: &Path, size_gb: u64) -> Result<()> {
        if output_path.exists() {
            self.msg(&format!(
                "Image file already exists: {}",
                output_path.display()
            ));
            return Ok(());
        }

        self.msg(&format!(
            "Creating {}GB image file: {}",
            size_gb,
            output_path.display()
        ));

        let size_str = format!("{}G", size_gb);
        let output = self.executor.execute(
            "fallocate",
            &[
                "-l".to_string(),
                size_str,
                output_path.to_string_lossy().to_string(),
            ],
        )?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to create image file with fallocate. Exit code: {:?}",
                output.status.code()
            ));
        }

        Ok(())
    }

    /// Inject trellis configuration into an installed disk image.
    ///
    /// This mounts the disk image's root partition and writes the trellis
    /// configuration files directly to the installed system.
    ///
    /// # Arguments
    ///
    /// * `disk_image_path` - Path to the bootable disk image file
    ///
    /// # Errors
    ///
    /// Returns an error if mounting, writing, or unmounting fails.
    pub fn inject_configuration_to_disk(&self, disk_image_path: &Path) -> Result<()> {
        self.msg("Injecting trellis configuration into disk image");

        // Generate the configuration TOML
        let mut image_config = Config::default();

        // Override with actual build values
        if let Some(build) = &mut image_config.build {
            build.builder_stages = Some(self.config.builder_stages.clone());
            build.rootfs_stages = Some(self.config.rootfs_stages.clone());
            build.builder_tag = Some(self.config.builder_tag.clone());
            build.rootfs_tag = Some(self.config.rootfs_tag.clone());
        }

        if let Some(environment) = &mut image_config.environment {
            environment.stages_dir = Some(PathBuf::from("/var/lib/trellis/stages"));
        }

        let toml_content =
            toml::to_string_pretty(&image_config).context("Failed to serialize configuration")?;

        // Set up the loopback device for the disk image
        let output = self.executor.execute(
            "losetup",
            &[
                "--find".to_string(),
                "--show".to_string(),
                "--partscan".to_string(),
                disk_image_path.to_string_lossy().to_string(),
            ],
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to set up loopback device: {}", stderr));
        }

        let loop_device = String::from_utf8_lossy(&output.stdout).trim().to_string();
        self.msg(&format!("Created loopback device: {}", loop_device));

        // The root partition is typically partition 3 (after EFI and boot)
        // GPT disk layout from bootc: p1=EFI, p2=boot, p3=root
        let root_partition = format!("{}p3", loop_device);

        // Create a temporary mount point
        let mount_point = std::env::temp_dir().join(format!(
            "trellis-mount-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&mount_point).context("Failed to create mount point")?;

        // Function to ensure cleanup happens
        let cleanup = |executor: &dyn CommandExecutor, mount_point: &Path, loop_device: &str| {
            // Unmount
            let _ = executor.execute("umount", &[mount_point.to_string_lossy().to_string()]);
            // Remove mount point
            let _ = std::fs::remove_dir(mount_point);
            // Detach loopback device
            let _ = executor.execute("losetup", &["-d".to_string(), loop_device.to_string()]);
        };

        // Mount the root partition
        let output = self.executor.execute(
            "mount",
            &[
                root_partition.clone(),
                mount_point.to_string_lossy().to_string(),
            ],
        )?;

        if !output.status.success() {
            cleanup(self.executor.as_ref(), &mount_point, &loop_device);
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Failed to mount root partition {}: {}",
                root_partition,
                stderr
            ));
        }

        self.msg(&format!("Mounted {} at {}", root_partition, mount_point.display()));

        // Create trellis directories
        let trellis_config_dir = mount_point.join("etc/trellis");
        let trellis_stages_dir = mount_point.join("var/lib/trellis/stages");

        if let Err(e) = std::fs::create_dir_all(&trellis_config_dir) {
            cleanup(self.executor.as_ref(), &mount_point, &loop_device);
            return Err(anyhow!("Failed to create config directory: {}", e));
        }

        if let Err(e) = std::fs::create_dir_all(&trellis_stages_dir) {
            cleanup(self.executor.as_ref(), &mount_point, &loop_device);
            return Err(anyhow!("Failed to create stages directory: {}", e));
        }

        // Write trellis.toml
        let config_path = trellis_config_dir.join("trellis.toml");
        if let Err(e) = std::fs::write(&config_path, &toml_content) {
            cleanup(self.executor.as_ref(), &mount_point, &loop_device);
            return Err(anyhow!("Failed to write trellis.toml: {}", e));
        }
        self.msg(&format!("Wrote configuration to {}", config_path.display()));

        // Copy stages directory if it exists
        if self.config.stages_dir.exists() {
            self.msg(&format!(
                "Copying stages from {} to disk image",
                self.config.stages_dir.display()
            ));

            // Use cp to copy the stages directory contents
            let output = self.executor.execute(
                "cp",
                &[
                    "-r".to_string(),
                    format!("{}/.", self.config.stages_dir.display()),
                    trellis_stages_dir.to_string_lossy().to_string(),
                ],
            )?;

            if !output.status.success() {
                cleanup(self.executor.as_ref(), &mount_point, &loop_device);
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("Failed to copy stages: {}", stderr));
            }
        }

        // Sync to ensure all writes are flushed
        let _ = self.executor.execute("sync", &[]);

        // Clean up: unmount and detach loopback
        let output = self.executor.execute(
            "umount",
            &[mount_point.to_string_lossy().to_string()],
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.warning(&format!("Failed to unmount cleanly: {}", stderr));
        }

        let _ = std::fs::remove_dir(&mount_point);

        let output = self.executor.execute(
            "losetup",
            &["-d".to_string(), loop_device.clone()],
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.warning(&format!("Failed to detach loopback device: {}", stderr));
        }

        self.msg("Configuration injected successfully");
        Ok(())
    }

    /// Install the bootable system using bootc.
    pub fn install_bootable_system(
        &self,
        image_tag: &str,
        output_path: &Path,
        filesystem: &str,
    ) -> Result<()> {
        self.msg("Installing bootable system with bootc");
        let output_dir = output_path.parent().context("Invalid output path")?;
        let filename = output_path.file_name().context("Invalid output filename")?;

        // Ensure required filesystem tools exist in the image (mkfs.fat or mkfs.vfat)
        let check_cmd = "which mkfs.fat || which mkfs.vfat".to_string();
        let check_output = self.executor.execute(
            "podman",
            &[
                "run".to_string(),
                "--rm".to_string(),
                image_tag.to_string(),
                "sh".to_string(),
                "-c".to_string(),
                check_cmd,
            ],
        )?;
        let stdout = String::from_utf8_lossy(&check_output.stdout);
        if !check_output.status.success() || stdout.trim().is_empty() {
            return Err(anyhow!(
                "Required filesystem tool mkfs.fat (or mkfs.vfat) not found in image '{}'. Please install dosfstools or provide an image with mkfs.fat available.",
                image_tag
            ));
        }

        // Build the full podman run command including the bootc command at the end
        let mut run_args = vec![
            "--rm".to_string(),
            "--privileged".to_string(),
            "--pid=host".to_string(),
        ];

        // Add SELinux bind mount only if the host path exists
        if std::path::Path::new("/sys/fs/selinux").exists() {
            run_args.push("-v".to_string());
            run_args.push("/sys/fs/selinux:/sys/fs/selinux".to_string());
        } else {
            self.warning("Host path /sys/fs/selinux not found; skipping SELinux mount");
        }

        run_args.extend(vec![
            "-v".to_string(),
            "/etc/containers:/etc/containers:Z".to_string(),
            "-v".to_string(),
            "/var/lib/containers:/var/lib/containers:Z".to_string(),
            "-v".to_string(),
            "/dev:/dev".to_string(),
            "-v".to_string(),
            format!("{}:/data", output_dir.display()),
            "--security-opt".to_string(),
            "label=type:unconfined_t".to_string(),
            image_tag.to_string(),
            // Bootc command and args
            "bootc".to_string(),
            "install".to_string(),
            "to-disk".to_string(),
            "--composefs-backend".to_string(),
            "--via-loopback".to_string(),
            format!("/data/{}", filename.to_string_lossy()),
            "--filesystem".to_string(),
            filesystem.to_string(),
            "--wipe".to_string(),
            "--bootloader".to_string(),
            "systemd".to_string(),
        ]);
        let status = if self.config.quiet {
            let output = self.executor.podman_run(&run_args)?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("Bootc install failed: {}", stderr));
            }
            output.status
        } else {
            self.executor.podman_run_streaming(&run_args)?
        };
        if !status.success() {
            return Err(anyhow!(
                "Bootc install failed with exit code: {:?}",
                status.code()
            ));
        }
        Ok(())
    }
}

impl<'a> TrellisMessaging for ImageGenerator<'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Create a minimal TrellisConfig for testing.
    fn create_test_config() -> TrellisConfig {
        TrellisConfig {
            builder_stages: vec!["base".to_string()],
            builder_tag: "test-builder".to_string(),
            podman_build_cache: false,
            auto_clean: false,
            pacman_cache: None,
            aur_cache: None,
            stages_dir: PathBuf::from("/tmp"),
            rootfs_stages: vec!["base".to_string()],
            rootfs_base: "scratch".to_string(),
            extra_contexts: vec![],
            extra_mounts: vec![],
            rootfs_tag: "trellis-rootfs".to_string(),
            hooks_dir: None,
            quiet: false,
        }
    }

    #[test]
    fn resolve_image_tag_full_path_with_version() {
        let config = create_test_config();
        let result = resolve_image_tag(&config, Some("registry.io/my-image:v1"));
        assert_eq!(result, "registry.io/my-image:v1");
    }

    #[test]
    fn resolve_image_tag_full_path_no_version() {
        let config = create_test_config();
        let result = resolve_image_tag(&config, Some("registry.io/my-image"));
        assert_eq!(result, "registry.io/my-image:latest");
    }

    #[test]
    fn resolve_image_tag_with_version_no_registry() {
        let config = create_test_config();
        let result = resolve_image_tag(&config, Some("my-image:v1"));
        assert_eq!(result, "localhost/my-image:v1");
    }

    #[test]
    fn resolve_image_tag_name_only() {
        let config = create_test_config();
        let result = resolve_image_tag(&config, Some("my-image"));
        assert_eq!(result, "localhost/my-image:latest");
    }

    #[test]
    fn resolve_image_tag_none_uses_config() {
        let config = create_test_config();
        let result = resolve_image_tag(&config, None);
        assert_eq!(result, "localhost/trellis-rootfs:latest");
    }

    #[test]
    fn resolve_image_tag_complex_registry() {
        let config = create_test_config();
        let result = resolve_image_tag(&config, Some("quay.io/org/my-image:latest"));
        assert_eq!(result, "quay.io/org/my-image:latest");
    }

    #[test]
    fn resolve_image_tag_complex_registry_no_tag() {
        let config = create_test_config();
        let result = resolve_image_tag(&config, Some("quay.io/org/my-image"));
        assert_eq!(result, "quay.io/org/my-image:latest");
    }
}
