use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    cli::Cli,
    trellis::constants::{containers, env_vars, paths},
};

use super::merger::{BoolMerger, ConfigMerger};
use super::validator::ConfigValidator;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub build: Option<BuildConfig>,
    pub environment: Option<EnvironmentConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BuildConfig {
    pub builder_stages: Option<Vec<String>>,
    pub rootfs_stages: Option<Vec<String>>,
    pub rootfs_base: Option<String>,
    pub builder_tag: Option<String>,
    pub rootfs_tag: Option<String>,
    pub podman_build_cache: Option<bool>,
    pub auto_clean: Option<bool>,
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
                rootfs_base: Some("scratch".to_string()),
                builder_tag: Some(containers::DEFAULT_BUILDER_TAG.to_string()),
                rootfs_tag: Some(containers::DEFAULT_ROOTFS_TAG.to_string()),
                podman_build_cache: Some(false),
                auto_clean: Some(false),
                extra_contexts: None,
                extra_mounts: None,
            }),
            environment: Some(EnvironmentConfig {
                pacman_cache: Some(PathBuf::from(paths::DEFAULT_PACMAN_CACHE)),
                aur_cache: Some(PathBuf::from(paths::DEFAULT_AUR_CACHE)),
                src_dir: None,
                hooks_dir: Some(PathBuf::from(paths::DEFAULT_HOOKS_DIR)),
            }),
        }
    }
}

#[derive(Debug)]
pub struct TrellisConfig {
    pub builder_stages: Vec<String>,
    pub builder_tag: String,
    pub podman_build_cache: bool,
    pub auto_clean: bool,
    pub pacman_cache: Option<PathBuf>,
    pub aur_cache: Option<PathBuf>,
    pub src_dir: PathBuf,
    pub rootfs_stages: Vec<String>,
    pub rootfs_base: String,
    pub extra_contexts: Vec<String>,
    pub extra_mounts: Vec<PathBuf>,
    pub rootfs_tag: String,
    pub hooks_dir: Option<PathBuf>,
    pub quiet: bool,
}

impl TrellisConfig {
    /// Creates a new TrellisConfig by merging CLI arguments with configuration file values.
    ///
    /// CLI arguments take precedence over configuration file values, which take precedence
    /// over default values. The configuration file path can be overridden with the
    /// `TRELLIS_CONFIG` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file exists but cannot be read or parsed.
    pub fn new(cli: Cli) -> Result<Self> {
        let config_file = env::var(env_vars::CONFIG_PATH)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(paths::DEFAULT_CONFIG_PATH));

        let file_config = if config_file.exists() {
            let content = fs::read_to_string(&config_file)
                .with_context(|| format!("Failed to read config file: {config_file:?}"))?;
            toml::from_str::<Config>(&content)
                .with_context(|| format!("Failed to parse config file: {config_file:?}"))?
        } else {
            Config::default()
        };

        let build_config = file_config.build.as_ref();
        let env_config = file_config.environment.as_ref();

        let config = TrellisConfig {
            builder_stages: Vec::merge(
                cli.builder_stages,
                Self::get_build_field(build_config, |b| &b.builder_stages),
                Vec::new(),
            ),
            rootfs_stages: Vec::merge(
                cli.rootfs_stages,
                Self::get_build_field(build_config, |b| &b.rootfs_stages),
                Vec::new(),
            ),
            rootfs_base: String::merge(
                cli.rootfs_base,
                Self::get_build_field(build_config, |b| &b.rootfs_base),
                "scratch".to_string(),
            ),
            extra_contexts: Vec::merge(
                cli.extra_contexts,
                Self::get_build_field(build_config, |b| &b.extra_contexts),
                Vec::new(),
            ),
            extra_mounts: Vec::merge(
                cli.extra_mounts,
                Self::get_build_field(build_config, |b| &b.extra_mounts),
                Vec::new(),
            ),
            builder_tag: String::merge(
                cli.builder_tag,
                Self::get_build_field(build_config, |b| &b.builder_tag),
                containers::DEFAULT_BUILDER_TAG.to_string(),
            ),
            rootfs_tag: String::merge(
                cli.rootfs_tag,
                Self::get_build_field(build_config, |b| &b.rootfs_tag),
                containers::DEFAULT_ROOTFS_TAG.to_string(),
            ),
            podman_build_cache: BoolMerger::merge(
                cli.podman_build_cache,
                build_config.and_then(|b| b.podman_build_cache),
                false,
            ),
            auto_clean: cli.auto_clean || build_config.and_then(|b| b.auto_clean).unwrap_or(false),
            pacman_cache: Option::merge(
                cli.pacman_cache,
                Some(Self::get_env_field(env_config, |e| &e.pacman_cache)),
                None,
            ),
            aur_cache: Option::merge(
                cli.aur_cache,
                Some(Self::get_env_field(env_config, |e| &e.aur_cache)),
                None,
            ),
            src_dir: cli
                .src_dir
                .or_else(|| env_config.and_then(|e| e.src_dir.clone()))
                .unwrap_or_else(|| PathBuf::from(paths::DEFAULT_SRC_DIR)),
            hooks_dir: Self::resolve_hooks_dir(env_config),
            quiet: cli.quiet,
        };

        // Validate the complete configuration
        ConfigValidator::validate_complete(&config)?;

        Ok(config)
    }

    /// Resolves the hooks directory with proper existence checking.
    fn resolve_hooks_dir(env_config: Option<&EnvironmentConfig>) -> Option<PathBuf> {
        let hooks_dir = env_config
            .and_then(|e| e.hooks_dir.clone())
            .unwrap_or_else(|| PathBuf::from(paths::DEFAULT_HOOKS_DIR));
        hooks_dir.exists().then_some(hooks_dir)
    }

    /// Helper function to consolidate build config field access patterns.
    fn get_build_field<T: Clone>(
        build_config: Option<&BuildConfig>,
        field_getter: fn(&BuildConfig) -> &Option<T>,
    ) -> Option<T> {
        build_config.and_then(|b| field_getter(b).clone())
    }

    /// Helper function to consolidate environment config field access patterns.
    fn get_env_field<T: Clone>(
        env_config: Option<&EnvironmentConfig>,
        field_getter: fn(&EnvironmentConfig) -> &Option<T>,
    ) -> Option<T> {
        env_config.and_then(|e| field_getter(e).clone())
    }
}
