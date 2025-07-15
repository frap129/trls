//! Trellis core functionality modules.
//! 
//! This module contains the main application logic split into focused components:
//! - `builder`: Container building operations
//! - `cleaner`: Image cleanup and management
//! - `runner`: Container execution
//! - `discovery`: Containerfile discovery logic

use anyhow::Result;

use crate::{
    cli::{Cli, Commands},
    config::TrellisConfig,
};

use common::TrellisMessaging;

pub mod builder;
pub mod cleaner;
pub mod runner;
pub mod discovery;
pub mod common;
pub mod constants;

pub use builder::{ContainerBuilder, validate_stages_not_empty};
pub use cleaner::ImageCleaner;
pub use runner::ContainerRunner;

/// Main application struct that coordinates all trellis operations.
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

/// Core trellis functionality coordinating all subsystems.
pub struct Trellis<'a> {
    config: &'a TrellisConfig,
    builder: ContainerBuilder<'a>,
    cleaner: ImageCleaner<'a>,
    runner: ContainerRunner,
}

impl<'a> TrellisMessaging for Trellis<'a> {}

impl<'a> Trellis<'a> {
    pub fn new(config: &'a TrellisConfig) -> Self {
        Trellis {
            config,
            builder: ContainerBuilder::new(config),
            cleaner: ImageCleaner::new(config),
            runner: ContainerRunner::new(config),
        }
    }

    pub fn build_builder_container(&self) -> Result<()> {
        validate_stages_not_empty(&self.config.builder_stages, "builder")?;

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
        validate_stages_not_empty(&self.config.rootfs_stages, "rootfs")?;

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
