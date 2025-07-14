use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::process::Command;

use crate::cli::{Cli, Commands};
use crate::config::TrellisConfig;


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
        println!("====> {}", message);
    }

    fn warning(&self, message: &str) {
        eprintln!("====> WARNING: {}", message);
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
            let (group, stage) = if build_stage.contains(':') {
                let parts: Vec<&str> = build_stage.split(':').collect();
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (build_stage.clone(), build_stage.clone())
            };

            let tag = if i == build_stages.len() - 1 {
                final_tag.to_string()
            } else if stage != group {
                format!("trellis-{}-{}-{}", tmp_name, group, stage)
            } else {
                format!("trellis-{}-{}", tmp_name, stage)
            };

            let containerfile_path = self.find_containerfile(&group)?;

            let args = vec![
                "--net=host".to_string(),
                "--cap-add".to_string(),
                "sys_admin".to_string(),
                "--cap-add".to_string(),
                "mknod".to_string(),
                "--squash".to_string(),
                "-f".to_string(),
                containerfile_path,
                "--build-arg".to_string(),
                format!("BASE_IMAGE=localhost/{}", last_stage),
                "--target".to_string(),
                stage,
                "-t".to_string(),
                tag.clone(),
            ];

            build_cmd_fn(self, &args)?;
            last_stage = tag;
        }

        Ok(())
    }

    fn find_containerfile(&self, group: &str) -> Result<String> {
        let filename = format!("Containerfile.{}", group);
        
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
        
        Err(anyhow!("Containerfile not found: {} (searched in {}/{} and {})", 
                   filename, 
                   self.config.src_dir.display(), 
                   group,
                   self.config.src_dir.display()))
    }

    fn builder_podman_cmd(&self, args: &[String]) -> Result<()> {
        let mut cmd = Command::new("podman");
        cmd.arg("build");

        if !self.config.podman_build_cache {
            cmd.arg("--no-cache");
            env::set_var("BUILDAH_LAYERS", "false");
        }

        cmd.args(args);

        let status = cmd.status()
            .context("Failed to execute podman build command")?;

        if !status.success() {
            return Err(anyhow!("Podman build failed"));
        }

        Ok(())
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

        // Add pacman cache
        if let Some(pacman_cache) = &self.config.pacman_cache {
            if fs::create_dir_all(pacman_cache).is_err() {
                self.warning(&format!("Failed to create pacman cache directory: {:?}", pacman_cache));
            } else {
                cmd.arg("-v").arg(format!("{}:/var/cache/pacman/pkg", pacman_cache.display()));
            }
        }

        // Add AUR cache
        if let Some(aur_cache) = &self.config.aur_cache {
            if fs::create_dir_all(aur_cache).is_err() {
                self.warning(&format!("Failed to create AUR cache directory: {:?}", aur_cache));
            } else {
                cmd.arg("-v").arg(format!("{}:/var/cache/trellis/aur", aur_cache.display()));
            }
        }

        // Add hooks directory
        if let Some(hooks_dir) = &self.config.hooks_dir {
            cmd.arg("-v").arg(format!("{}:{}", hooks_dir.display(), hooks_dir.display()));
            cmd.arg("--build-arg").arg(format!("HOOKS_DIR={}", hooks_dir.display()));
        }

        // Add extra mounts
        for mount in &self.config.extra_mounts {
            cmd.arg("-v").arg(format!("{}:{}", mount.display(), mount.display()));
        }

        cmd.args(args);

        let status = cmd.status()
            .context("Failed to execute podman build command")?;

        if !status.success() {
            return Err(anyhow!("Podman build failed"));
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
        cmd.args(&[
            "run",
            "--net=host",
            "--cap-add",
            "all",
            "--rm",
            "-it",
            &format!("localhost/{}", self.config.rootfs_tag),
        ]);
        cmd.args(args);

        let status = cmd.status()
            .context("Failed to execute podman run command")?;

        if !status.success() {
            return Err(anyhow!("Podman run failed"));
        }

        Ok(())
    }

    pub fn clean(&self) -> Result<()> {
        let status = Command::new("podman")
            .args(&["system", "prune"])
            .status()
            .context("Failed to execute podman system prune")?;

        if !status.success() {
            return Err(anyhow!("Podman system prune failed"));
        }

        self.msg("System cleaned successfully");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use crate::cli::{Cli, Commands};

    fn create_test_config(temp_dir: &TempDir) -> TrellisConfig {
        TrellisConfig {
            builder_stages: vec!["base".to_string()],
            builder_tag: "test-builder".to_string(),
            podman_build_cache: false,
            pacman_cache: None,
            aur_cache: None,
            src_dir: temp_dir.path().to_path_buf(),
            rootfs_stages: vec!["base".to_string(), "final".to_string()],
            extra_contexts: vec![],
            extra_mounts: vec![],
            rootfs_tag: "test-rootfs".to_string(),
            hooks_dir: None,
        }
    }

    #[test]
    fn test_find_containerfile_in_subdir() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().join("base");
        fs::create_dir_all(&base_dir).unwrap();
        
        let containerfile_path = base_dir.join("Containerfile.base");
        fs::write(&containerfile_path, "FROM alpine").unwrap();
        
        let config = create_test_config(&temp_dir);
        let trellis = Trellis::new(&config);
        
        let result = trellis.find_containerfile("base").unwrap();
        assert_eq!(result, containerfile_path.to_string_lossy());
    }

    #[test]
    fn test_find_containerfile_in_root() {
        let temp_dir = TempDir::new().unwrap();
        let containerfile_path = temp_dir.path().join("Containerfile.base");
        fs::write(&containerfile_path, "FROM alpine").unwrap();
        
        let config = create_test_config(&temp_dir);
        let trellis = Trellis::new(&config);
        
        let result = trellis.find_containerfile("base").unwrap();
        assert_eq!(result, containerfile_path.to_string_lossy());
    }

    #[test]
    fn test_find_containerfile_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let trellis = Trellis::new(&config);
        
        let result = trellis.find_containerfile("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Containerfile not found"));
    }

    #[test]
    fn test_find_containerfile_prefers_subdir() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create both versions
        let root_containerfile = temp_dir.path().join("Containerfile.base");
        fs::write(&root_containerfile, "FROM alpine:root").unwrap();
        
        let base_dir = temp_dir.path().join("base");
        fs::create_dir_all(&base_dir).unwrap();
        let subdir_containerfile = base_dir.join("Containerfile.base");
        fs::write(&subdir_containerfile, "FROM alpine:subdir").unwrap();
        
        let config = create_test_config(&temp_dir);
        let trellis = Trellis::new(&config);
        
        let result = trellis.find_containerfile("base").unwrap();
        assert_eq!(result, subdir_containerfile.to_string_lossy());
    }

    #[test]
    fn test_trellis_app_creation() {
        let cli = Cli {
            command: Commands::Build,
            builder_tag: "test-builder".to_string(),
            podman_build_cache: None,
            pacman_cache: None,
            aur_cache: None,
            src_dir: None,
            extra_contexts: vec![],
            extra_mounts: vec![],
            rootfs_stages: vec!["base".to_string()],
            rootfs_tag: "test-rootfs".to_string(),
            builder_stages: vec![],
        };
        
        let app = TrellisApp::new(cli);
        assert!(app.is_ok());
    }

    #[test]
    fn test_build_rootfs_container_no_stages() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);
        config.rootfs_stages = vec![];
        
        let trellis = Trellis::new(&config);
        let result = trellis.build_rootfs_container();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No rootfs stages defined"));
    }

    #[test]
    fn test_build_builder_container_no_stages() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(&temp_dir);
        config.builder_stages = vec![];
        
        let trellis = Trellis::new(&config);
        let result = trellis.build_builder_container();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No builder stages defined"));
    }
}
