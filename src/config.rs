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

        let script_dir = env::current_exe()
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
            .unwrap_or_else(|| script_dir.join("src"));

        let hooks_dir = PathBuf::from("/etc/trellis/hooks.d");

        Ok(TrellisConfig {
            builder_stages: get_builder_stages(),
            builder_tag: get_builder_tag().unwrap_or_else(|| "trellis-builder".to_string()),
            podman_build_cache: cli.podman_build_cache
                .or(get_podman_build_cache())
                .unwrap_or(false),
            pacman_cache: cli.pacman_cache.or(get_pacman_cache()),
            aur_cache: cli.aur_cache.or(get_aur_cache()),
            src_dir,
            rootfs_stages: get_rootfs_stages(),
            extra_contexts: get_extra_contexts(),
            extra_mounts: get_extra_mounts(),
            rootfs_tag: get_rootfs_tag().unwrap_or_else(|| "trellis-rootfs".to_string()),
            hooks_dir: if hooks_dir.exists() {
                Some(hooks_dir)
            } else {
                None
            },
        })
    }
}