use std::{env, fs, process::Command};

use anyhow::{anyhow, Context, Result};

use crate::{
    cli::{Cli, Commands},
    config::TrellisConfig,
};


pub struct TrellisApp {
    config: TrellisConfig,
    command: Commands,
}

impl TrellisApp {
    pub fn new(cli: Cli) -> Result<Self> {
        let command = cli.command.clone();
        let config = TrellisConfig::new(cli)?;
        
        Ok(TrellisApp { config, command })
    }

    pub fn run(&self) -> Result<()> {
        let trellis = Trellis::new(&self.config);
        
        match &self.command {
            Commands::BuildBuilder => trellis.build_builder_container(),
            Commands::Build => trellis.build_rootfs_container(),
            Commands::Run { args } => trellis.run_rootfs_container(args),
            Commands::Clean => trellis.clean(),
            Commands::Update => trellis.update(),
        }
    }
}

pub struct Trellis<'a> {
    config: &'a TrellisConfig,
}

impl<'a> Trellis<'a> {
    pub fn new(config: &'a TrellisConfig) -> Self {
        Trellis { config }
    }

    fn msg(&self, message: &str) {
        println!("====> {message}");
    }

    fn warning(&self, message: &str) {
        eprintln!("====> WARNING: {message}");
    }

    fn multistage_container_build(
        &self,
        tmp_name: &str,
        final_tag: &str,
        build_stages: &[String],
        build_cmd_fn: impl Fn(&Self, &[String]) -> Result<()>,
    ) -> Result<()> {
        let mut last_stage = String::new();

        for (i, build_stage) in build_stages.iter().enumerate() {
            let (group, stage) = Self::parse_stage_name(build_stage);

            let tag = if i == build_stages.len() - 1 {
                final_tag.to_string()
            } else if stage != group {
                format!("trellis-{tmp_name}-{group}-{stage}")
            } else {
                format!("trellis-{tmp_name}-{stage}")
            };

            let containerfile_path = self.find_containerfile(&group)?;

            let args = [
                "--net=host",
                "--cap-add",
                "sys_admin",
                "--cap-add",
                "mknod",
                "--squash",
                "-f",
                &containerfile_path,
                "--build-arg",
                &format!("BASE_IMAGE=localhost/{last_stage}"),
                "--target",
                &stage,
                "-t",
                &tag,
            ]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

            build_cmd_fn(self, &args)?;
            last_stage = tag;
        }

        Ok(())
    }

    fn parse_stage_name(build_stage: &str) -> (String, String) {
        if let Some((group, stage)) = build_stage.split_once(':') {
            (group.to_string(), stage.to_string())
        } else {
            (build_stage.to_string(), build_stage.to_string())
        }
    }

    pub fn find_containerfile(&self, group: &str) -> Result<String> {
        let filename = format!("Containerfile.{group}");
        
        // First try: look in subdirectory named after the group
        let subdir_path = self.config.src_dir.join(group).join(&filename);
        if subdir_path.exists() {
            return Ok(subdir_path.to_string_lossy().to_string());
        }
        
        // Second try: look in root src directory
        let root_path = self.config.src_dir.join(&filename);
        if root_path.exists() {
            return Ok(root_path.to_string_lossy().to_string());
        }
        
        Err(anyhow!(
            "Containerfile not found: {filename} (searched in {}/{group} and {})",
            self.config.src_dir.display(),
            self.config.src_dir.display()
        ))
    }

    fn builder_podman_cmd(&self, args: &[String]) -> Result<()> {
        let mut cmd = Command::new("podman");
        cmd.arg("build");

        if !self.config.podman_build_cache {
            cmd.arg("--no-cache");
            env::set_var("BUILDAH_LAYERS", "false");
        }

        cmd.args(args);

        let status = cmd
            .status()
            .context("Failed to execute podman build command")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("Podman build failed"))
        }
    }

    fn rootfs_podman_cmd(&self, args: &[String]) -> Result<()> {
        let mut cmd = Command::new("podman");
        cmd.arg("build");

        if !self.config.podman_build_cache {
            cmd.arg("--no-cache");
            env::set_var("BUILDAH_LAYERS", "false");
        }

        // Add build contexts
        for context in &self.config.extra_contexts {
            cmd.arg("--build-context").arg(context);
        }

        // Add cache directories
        self.add_cache_mount(&mut cmd, &self.config.pacman_cache, "pacman", "/var/cache/pacman/pkg")?;
        self.add_cache_mount(&mut cmd, &self.config.aur_cache, "AUR", "/var/cache/trellis/aur")?;

        // Add hooks directory
        if let Some(hooks_dir) = &self.config.hooks_dir {
            let hooks_path = hooks_dir.display().to_string();
            cmd.arg("-v").arg(format!("{hooks_path}:{hooks_path}"));
            cmd.arg("--build-arg").arg(format!("HOOKS_DIR={hooks_path}"));
        }

        // Add extra mounts
        for mount in &self.config.extra_mounts {
            let mount_path = mount.display().to_string();
            cmd.arg("-v").arg(format!("{mount_path}:{mount_path}"));
        }

        cmd.args(args);

        let status = cmd
            .status()
            .context("Failed to execute podman build command")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("Podman build failed"))
        }
    }

    fn add_cache_mount(
        &self,
        cmd: &mut Command,
        cache_path: &Option<std::path::PathBuf>,
        cache_name: &str,
        container_path: &str,
    ) -> Result<()> {
        if let Some(cache_dir) = cache_path {
            if let Err(e) = fs::create_dir_all(cache_dir) {
                self.warning(&format!(
                    "Failed to create {cache_name} cache directory: {cache_dir:?} - {e}"
                ));
            } else {
                cmd.arg("-v").arg(format!("{}:{container_path}", cache_dir.display()));
            }
        }
        Ok(())
    }

    pub fn build_builder_container(&self) -> Result<()> {
        if self.config.builder_stages.is_empty() {
            return Err(anyhow!("No builder stages defined"));
        }

        self.multistage_container_build(
            "builder",
            &self.config.builder_tag,
            &self.config.builder_stages,
            |trellis, args| trellis.builder_podman_cmd(args),
        )?;

        self.msg("Builder container built successfully");
        Ok(())
    }

    pub fn build_rootfs_container(&self) -> Result<()> {
        if self.config.rootfs_stages.is_empty() {
            return Err(anyhow!("No rootfs stages defined"));
        }

        self.multistage_container_build(
            "stage",
            &self.config.rootfs_tag,
            &self.config.rootfs_stages,
            |trellis, args| trellis.rootfs_podman_cmd(args),
        )?;

        self.msg("Rootfs container built successfully");
        Ok(())
    }

    pub fn run_rootfs_container(&self, args: &[String]) -> Result<()> {
        let mut cmd = Command::new("podman");
        cmd.args([
            "run",
            "--net=host",
            "--cap-add",
            "all",
            "--rm",
            "-it",
            &format!("localhost/{}", self.config.rootfs_tag),
        ]);
        cmd.args(args);

        let status = cmd
            .status()
            .context("Failed to execute podman run command")?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("Podman run failed"))
        }
    }

    fn is_trellis_image(&self, image: &str) -> bool {
        // Pre-compute expected image names to avoid repeated allocations
        let expected_builder = format!("localhost/{}:latest", self.config.builder_tag);
        let expected_rootfs = format!("localhost/{}:latest", self.config.rootfs_tag);
        
        image == expected_builder ||
        image == expected_rootfs ||
        image.starts_with("localhost/trellis-builder") ||
        image.starts_with("localhost/trellis-stage")
    }

    pub fn clean(&self) -> Result<()> {
        self.msg("Cleaning trls-generated images...");
        
        let output = Command::new("podman")
            .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
            .output()
            .context("Failed to list podman images")?;
        
        if !output.status.success() {
            return Err(anyhow!("Failed to list images"));
        }
        
        let image_list = String::from_utf8_lossy(&output.stdout);
        let images_to_remove: Vec<&str> = image_list
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && self.is_trellis_image(line))
            .collect();
        
        if images_to_remove.is_empty() {
            self.msg("No trls-generated images found to clean");
            return Ok(());
        }
        
        self.msg(&format!("Found {} trls-generated images to remove", images_to_remove.len()));
        
        let mut removed_count = 0;
        for image in images_to_remove {
            let status = Command::new("podman")
                .args(["rmi", "-f", image])
                .status()
                .context("Failed to remove image")?;
                
            if status.success() {
                self.msg(&format!("Removed image: {}", image));
                removed_count += 1;
            } else {
                self.warning(&format!("Failed to remove image: {}", image));
            }
        }
        
        self.msg(&format!("Cleanup completed - removed {} images", removed_count));
        Ok(())
    }

    pub fn update(&self) -> Result<()> {
        self.build_rootfs_container()?;
        
        let status = Command::new("bootc")
            .arg("upgrade")
            .status()
            .context("Failed to execute bootc upgrade")?;

        if !status.success() {
            return Err(anyhow!("bootc upgrade failed"));
        }

        self.msg("Update completed successfully");
        Ok(())
    }
}

