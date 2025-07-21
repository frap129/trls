//! Trellis core functionality modules.
//!
//! This module contains the main application logic split into focused components:
//! - `builder`: Container building operations
//! - `cleaner`: Image cleanup and management
//! - `runner`: Container execution
//! - `discovery`: Containerfile discovery logic

use anyhow::Result;
use std::sync::Arc;

use crate::{
    cli::{Cli, Commands},
    config::{ConfigValidator, TrellisConfig},
};

use common::TrellisMessaging;
use executor::{CommandExecutor, RealCommandExecutor};

pub mod builder;
pub mod cleaner;
pub mod common;
pub mod constants;
pub mod discovery;
pub mod executor;
pub mod runner;

pub use builder::ContainerBuilder;
pub use cleaner::ImageCleaner;
pub use runner::ContainerRunner;

/// Main application struct that coordinates all trellis operations.
pub struct TrellisApp {
    config: TrellisConfig,
    command: Commands,
    executor: Arc<dyn CommandExecutor>,
}

impl std::fmt::Debug for TrellisApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrellisApp")
            .field("config", &self.config)
            .field("command", &self.command)
            .field("executor", &"<dyn CommandExecutor>")
            .finish()
    }
}

impl TrellisApp {
    pub fn new(cli: Cli) -> Result<Self> {
        let command = cli.command.clone();
        let config = TrellisConfig::new(cli)?;
        let executor = Arc::new(RealCommandExecutor::new());

        Ok(TrellisApp {
            config,
            command,
            executor,
        })
    }

    /// Create TrellisApp with custom executor for testing.
    pub fn with_executor(cli: Cli, executor: Arc<dyn CommandExecutor>) -> Result<Self> {
        let command = cli.command.clone();
        let config = TrellisConfig::new(cli)?;

        Ok(TrellisApp {
            config,
            command,
            executor,
        })
    }

    pub fn run(&self) -> Result<()> {
        let trellis = Trellis::new(&self.config, Arc::clone(&self.executor));

        match &self.command {
            Commands::BuildBuilder => trellis.build_builder_container(),
            Commands::Build => trellis.build_rootfs_container(),
            Commands::Run { args } => trellis.run_rootfs_container(args),
            Commands::Clean => trellis.clean(),
            Commands::Update => trellis.update(),
        }
    }
}

/// Core trellis functionality coordinating all subsystems.
pub struct Trellis<'a> {
    config: &'a TrellisConfig,
    builder: ContainerBuilder<'a>,
    cleaner: ImageCleaner<'a>,
    runner: ContainerRunner,
    executor: Arc<dyn CommandExecutor>,
}

impl<'a> TrellisMessaging for Trellis<'a> {}

impl<'a> Trellis<'a> {
    pub fn new(config: &'a TrellisConfig, executor: Arc<dyn CommandExecutor>) -> Self {
        Trellis {
            config,
            builder: ContainerBuilder::new(config, Arc::clone(&executor)),
            cleaner: ImageCleaner::new(config, Arc::clone(&executor)),
            runner: ContainerRunner::new(config, Arc::clone(&executor)),
            executor,
        }
    }

    pub fn build_builder_container(&self) -> Result<()> {
        ConfigValidator::validate_stages(&self.config.builder_stages, "builder")?;

        self.builder.build_multistage_container(
            "builder",
            &self.config.builder_tag,
            &self.config.builder_stages,
            builder::BuildType::Builder,
        )?;

        self.msg("Builder container built successfully");

        // Auto-clean intermediate images if enabled
        self.cleaner.auto_clean()?;

        Ok(())
    }

    pub fn build_rootfs_container(&self) -> Result<()> {
        ConfigValidator::validate_stages(&self.config.rootfs_stages, "rootfs")?;

        self.builder.build_multistage_container(
            "stage",
            &self.config.rootfs_tag,
            &self.config.rootfs_stages,
            builder::BuildType::Rootfs,
        )?;

        self.msg("Rootfs container built successfully");

        // Auto-clean intermediate images if enabled
        self.cleaner.auto_clean()?;

        Ok(())
    }

    pub fn run_rootfs_container(&self, args: &[String]) -> Result<()> {
        self.runner.run_container(&self.config.rootfs_tag, args)
    }

    pub fn clean(&self) -> Result<()> {
        self.cleaner.clean_all()
    }

    pub fn update(&self) -> Result<()> {
        self.build_rootfs_container()?;
        self.runner.run_bootc_upgrade()
    }
}
