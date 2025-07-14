use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::cli::Cli;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub build: Option<BuildConfig>,
    pub environment: Option<EnvironmentConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BuildConfig {
    pub builder_stages: Option<Vec<String>>,
    pub rootfs_stages: Option<Vec<String>>,
    pub builder_tag: Option<String>,
    pub rootfs_tag: Option<String>,
    pub podman_build_cache: Option<bool>,
    pub extra_contexts: Option<Vec<String>>,
    pub extra_mounts: Option<Vec<PathBuf>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentConfig {
    pub pacman_cache: Option<PathBuf>,
    pub aur_cache: Option<PathBuf>,
    pub src_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            build: Some(BuildConfig {
                builder_stages: None,
                rootfs_stages: None,
                builder_tag: Some("trellis-builder".to_string()),
                rootfs_tag: Some("trellis-rootfs".to_string()),
                podman_build_cache: Some(false),
                extra_contexts: None,
                extra_mounts: None,
            }),
            environment: Some(EnvironmentConfig {
                pacman_cache: Some(PathBuf::from("/var/cache/pacman/pkg")),
                aur_cache: Some(PathBuf::from("/var/cache/trellis/aur")),
                src_dir: None,
            }),
        }
    }
}

pub struct TrellisConfig {
    pub builder_stages: Vec<String>,
    pub builder_tag: String,
    pub podman_build_cache: bool,
    pub pacman_cache: Option<PathBuf>,
    pub aur_cache: Option<PathBuf>,
    pub src_dir: PathBuf,
    pub rootfs_stages: Vec<String>,
    pub extra_contexts: Vec<String>,
    pub extra_mounts: Vec<PathBuf>,
    pub rootfs_tag: String,
    pub hooks_dir: Option<PathBuf>,
}

impl TrellisConfig {
    pub fn new(cli: Cli) -> Result<Self> {
        let config_file = PathBuf::from("/etc/trellis/trellis.toml");
        let file_config = if config_file.exists() {
            let content = fs::read_to_string(&config_file)
                .with_context(|| format!("Failed to read config file: {:?}", config_file))?;
            toml::from_str::<Config>(&content)
                .with_context(|| "Failed to parse config file")?
        } else {
            Config::default()
        };

        let _script_dir = env::current_exe()
            .context("Failed to get current executable path")?
            .parent()
            .unwrap()
            .to_path_buf();

        // Helper functions to get values from nested config structure
        let get_builder_stages = || {
            if !cli.builder_stages.is_empty() {
                cli.builder_stages.clone()
            } else {
                file_config.build.as_ref()
                    .and_then(|b| b.builder_stages.clone())
                    .unwrap_or_default()
            }
        };

        let get_rootfs_stages = || {
            if !cli.rootfs_stages.is_empty() {
                cli.rootfs_stages.clone()
            } else {
                file_config.build.as_ref()
                    .and_then(|b| b.rootfs_stages.clone())
                    .unwrap_or_default()
            }
        };

        let get_builder_tag = || {
            file_config.build.as_ref()
                .and_then(|b| b.builder_tag.clone())
        };

        let get_rootfs_tag = || {
            file_config.build.as_ref()
                .and_then(|b| b.rootfs_tag.clone())
        };

        let get_podman_build_cache = || {
            file_config.build.as_ref()
                .and_then(|b| b.podman_build_cache)
        };

        let get_pacman_cache = || {
            file_config.environment.as_ref()
                .and_then(|e| e.pacman_cache.clone())
        };

        let get_aur_cache = || {
            file_config.environment.as_ref()
                .and_then(|e| e.aur_cache.clone())
        };

        let get_src_dir = || {
            file_config.environment.as_ref()
                .and_then(|e| e.src_dir.clone())
        };

        let get_extra_contexts = || {
            if !cli.extra_contexts.is_empty() {
                cli.extra_contexts.clone()
            } else {
                file_config.build.as_ref()
                    .and_then(|b| b.extra_contexts.clone())
                    .unwrap_or_default()
            }
        };

        let get_extra_mounts = || {
            if !cli.extra_mounts.is_empty() {
                cli.extra_mounts.clone()
            } else {
                file_config.build.as_ref()
                    .and_then(|b| b.extra_mounts.clone())
                    .unwrap_or_default()
            }
        };

        let src_dir = cli.src_dir
            .or(get_src_dir())
            .unwrap_or_else(|| PathBuf::from("/var/lib/trellis/src"));

        let hooks_dir = PathBuf::from("/etc/trellis/hooks.d");

        Ok(TrellisConfig {
            builder_stages: get_builder_stages(),
            builder_tag: if cli.builder_tag != "trellis-builder" {
                cli.builder_tag
            } else {
                get_builder_tag().unwrap_or_else(|| "trellis-builder".to_string())
            },
            podman_build_cache: cli.podman_build_cache
                .or(get_podman_build_cache())
                .unwrap_or(false),
            pacman_cache: cli.pacman_cache.or(get_pacman_cache()),
            aur_cache: cli.aur_cache.or(get_aur_cache()),
            src_dir,
            rootfs_stages: get_rootfs_stages(),
            extra_contexts: get_extra_contexts(),
            extra_mounts: get_extra_mounts(),
            rootfs_tag: if cli.rootfs_tag != "trellis-rootfs" {
                cli.rootfs_tag
            } else {
                get_rootfs_tag().unwrap_or_else(|| "trellis-rootfs".to_string())
            },
            hooks_dir: if hooks_dir.exists() {
                Some(hooks_dir)
            } else {
                None
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use crate::cli::{Cli, Commands};

    fn create_test_cli() -> Cli {
        Cli {
            command: Commands::Build,
            builder_tag: "test-builder".to_string(),
            podman_build_cache: None,
            pacman_cache: None,
            aur_cache: None,
            src_dir: None,
            extra_contexts: vec![],
            extra_mounts: vec![],
            rootfs_stages: vec![],
            rootfs_tag: "test-rootfs".to_string(),
            builder_stages: vec![],
        }
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.build.is_some());
        assert!(config.environment.is_some());
        
        let build = config.build.unwrap();
        assert_eq!(build.builder_tag, Some("trellis-builder".to_string()));
        assert_eq!(build.rootfs_tag, Some("trellis-rootfs".to_string()));
        assert_eq!(build.podman_build_cache, Some(false));
    }

    #[test]
    fn test_trellis_config_with_defaults() {
        let cli = create_test_cli();
        let config = TrellisConfig::new(cli).unwrap();
        
        // CLI values should override defaults when they differ from defaults
        assert_eq!(config.builder_tag, "test-builder");
        assert_eq!(config.rootfs_tag, "test-rootfs");
        assert!(!config.podman_build_cache);
        
        // Note: If system config file exists, it will provide default stages
        // This test validates that CLI overrides work even with system config
        // The actual stages depend on whether /etc/trellis/trellis.toml exists
    }

    #[test]
    fn test_trellis_config_with_cli_overrides() {
        let mut cli = create_test_cli();
        cli.builder_stages = vec!["stage1".to_string(), "stage2".to_string()];
        cli.rootfs_stages = vec!["base".to_string(), "final".to_string()];
        cli.podman_build_cache = Some(true);
        
        let config = TrellisConfig::new(cli).unwrap();
        
        assert_eq!(config.builder_stages, vec!["stage1", "stage2"]);
        assert_eq!(config.rootfs_stages, vec!["base", "final"]);
        assert!(config.podman_build_cache);
    }

    #[test]
    fn test_trellis_config_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("trellis.toml");
        
        let config_content = r#"
[build]
builder_stages = ["builder1", "builder2"]
rootfs_stages = ["base", "apps"]
builder_tag = "file-builder"
rootfs_tag = "file-rootfs"
podman_build_cache = true

[environment]
src_dir = "/custom/src"
pacman_cache = "/custom/pacman"
aur_cache = "/custom/aur"
"#;
        
        fs::write(&config_path, config_content).unwrap();
        
        // Note: This test would need modification to support custom config paths
        // For now, it demonstrates the config parsing logic
        let parsed: Config = toml::from_str(config_content).unwrap();
        
        assert!(parsed.build.is_some());
        let build = parsed.build.unwrap();
        assert_eq!(build.builder_stages, Some(vec!["builder1".to_string(), "builder2".to_string()]));
        assert_eq!(build.rootfs_stages, Some(vec!["base".to_string(), "apps".to_string()]));
        assert_eq!(build.builder_tag, Some("file-builder".to_string()));
        assert_eq!(build.podman_build_cache, Some(true));
    }
}