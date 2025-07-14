use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
    pub hooks_dir: Option<PathBuf>,
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
                hooks_dir: Some(PathBuf::from("/etc/trellis/hooks.d")),
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
                .with_context(|| format!("Failed to read config file: {config_file:?}"))?;
            toml::from_str::<Config>(&content)
                .with_context(|| "Failed to parse config file")?
        } else {
            Config::default()
        };


        // Extract configuration with CLI overrides
        let builder_stages = if !cli.builder_stages.is_empty() {
            cli.builder_stages
        } else {
            file_config
                .build
                .as_ref()
                .and_then(|b| b.builder_stages.clone())
                .unwrap_or_default()
        };

        let rootfs_stages = if !cli.rootfs_stages.is_empty() {
            cli.rootfs_stages
        } else {
            file_config
                .build
                .as_ref()
                .and_then(|b| b.rootfs_stages.clone())
                .unwrap_or_default()
        };

        let extra_contexts = if !cli.extra_contexts.is_empty() {
            cli.extra_contexts
        } else {
            file_config
                .build
                .as_ref()
                .and_then(|b| b.extra_contexts.clone())
                .unwrap_or_default()
        };

        let extra_mounts = if !cli.extra_mounts.is_empty() {
            cli.extra_mounts
        } else {
            file_config
                .build
                .as_ref()
                .and_then(|b| b.extra_mounts.clone())
                .unwrap_or_default()
        };

        let src_dir = cli
            .src_dir
            .or_else(|| {
                file_config
                    .environment
                    .as_ref()
                    .and_then(|e| e.src_dir.clone())
            })
            .unwrap_or_else(|| PathBuf::from("/var/lib/trellis/src"));

        let hooks_dir = file_config
            .environment
            .as_ref()
            .and_then(|e| e.hooks_dir.clone())
            .unwrap_or_else(|| PathBuf::from("/etc/trellis/hooks.d"));

        let builder_tag = if cli.builder_tag != "trellis-builder" {
            cli.builder_tag
        } else {
            file_config
                .build
                .as_ref()
                .and_then(|b| b.builder_tag.clone())
                .unwrap_or_else(|| "trellis-builder".to_string())
        };

        let rootfs_tag = if cli.rootfs_tag != "trellis-rootfs" {
            cli.rootfs_tag
        } else {
            file_config
                .build
                .as_ref()
                .and_then(|b| b.rootfs_tag.clone())
                .unwrap_or_else(|| "trellis-rootfs".to_string())
        };

        let podman_build_cache = cli
            .podman_build_cache
            .or_else(|| {
                file_config
                    .build
                    .as_ref()
                    .and_then(|b| b.podman_build_cache)
            })
            .unwrap_or(false);

        let pacman_cache = cli.pacman_cache.or_else(|| {
            file_config
                .environment
                .as_ref()
                .and_then(|e| e.pacman_cache.clone())
        });

        let aur_cache = cli.aur_cache.or_else(|| {
            file_config
                .environment
                .as_ref()
                .and_then(|e| e.aur_cache.clone())
        });

        Ok(TrellisConfig {
            builder_stages,
            builder_tag,
            podman_build_cache,
            pacman_cache,
            aur_cache,
            src_dir,
            rootfs_stages,
            extra_contexts,
            extra_mounts,
            rootfs_tag,
            hooks_dir: hooks_dir.exists().then_some(hooks_dir),
        })
    }
}

