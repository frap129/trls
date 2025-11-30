//! Trellis core functionality modules.
//!
//! This module contains the main application logic split into focused components:
//! - `builder`: Container building operations
//! - `cleaner`: Image cleanup and management
//! - `runner`: Container execution
//! - `discovery`: Containerfile discovery logic

use anyhow::{anyhow, Context, Result};
use std::sync::Arc;

use crate::{
    cli::{Cli, Commands},
    config::{ConfigValidator, TrellisConfig},
};

use common::TrellisMessaging;
use executor::{CommandExecutor, RealCommandExecutor};
use std::io::{self, BufRead};

/// Trait for handling user interactions like prompts and confirmations.
/// This allows for dependency injection and mocking in tests.
pub trait UserInteraction: Send + Sync {
    /// Prompts the user with a yes/no question.
    ///
    /// # Arguments
    ///
    /// * `message` - The prompt message to display to the user
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if the user responds affirmatively (y/yes)
    /// * `Ok(false)` if the user responds negatively or with any other input
    /// * `Err` if there's an error reading input
    fn prompt_yes_no(&self, message: &str) -> Result<bool>;
}

/// Real implementation of UserInteraction that reads from stdin.
pub struct RealUserInteraction;

impl UserInteraction for RealUserInteraction {
    fn prompt_yes_no(&self, message: &str) -> Result<bool> {
        eprint!("{message}");

        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut input = String::new();
        handle
            .read_line(&mut input)
            .context("Failed to read user input")?;

        let response = input.trim().to_lowercase();
        Ok(response == "y" || response == "yes")
    }
}

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
    #[allow(dead_code)]
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
        let user_interaction = Arc::new(RealUserInteraction);
        self.run_with_user_interaction(user_interaction)
    }

    /// Run with custom user interaction for testing.
    #[allow(dead_code)]
    pub fn run_with_user_interaction(
        &self,
        user_interaction: Arc<dyn UserInteraction>,
    ) -> Result<()> {
        let trellis = Trellis::new(&self.config, Arc::clone(&self.executor), user_interaction);

        match &self.command {
            Commands::BuildBuilder => trellis.build_builder_container(),
            Commands::Build => trellis.build_rootfs_container(),
            Commands::Run { args } => trellis.run_rootfs_container(args),
            Commands::Clean => trellis.clean(),
            Commands::Update => trellis.update(),
            Commands::QuickUpdate => trellis.quick_update_rootfs(),
        }
    }
}

/// Core trellis functionality coordinating all subsystems.
pub struct Trellis<'a> {
    config: &'a TrellisConfig,
    builder: ContainerBuilder<'a>,
    cleaner: ImageCleaner<'a>,
    runner: ContainerRunner<'a>,
    #[allow(dead_code)]
    executor: Arc<dyn CommandExecutor>,
    user_interaction: Arc<dyn UserInteraction>,
}

impl<'a> TrellisMessaging for Trellis<'a> {}

impl<'a> Trellis<'a> {
    pub fn new(
        config: &'a TrellisConfig,
        executor: Arc<dyn CommandExecutor>,
        user_interaction: Arc<dyn UserInteraction>,
    ) -> Self {
        Trellis {
            config,
            builder: ContainerBuilder::new(config, Arc::clone(&executor)),
            cleaner: ImageCleaner::new(config, Arc::clone(&executor)),
            runner: ContainerRunner::new(config, Arc::clone(&executor)),
            executor,
            user_interaction,
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

        // Check if builder container exists before building rootfs
        if !self.check_builder_container_exists()? {
            self.warning(&format!(
                "Builder container '{}' not found",
                self.config.builder_tag
            ));
            self.warning("The builder container is required for rootfs builds");

            if self
                .user_interaction
                .prompt_yes_no("Would you like to build it now? [y/N]: ")?
            {
                self.msg("Building builder container...");
                self.build_builder_container()?;
            }
        }

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

    /// Performs a quick update of the rootfs container using topgrade.
    pub fn quick_update_rootfs(&self) -> Result<()> {
        self.runner.quick_update_rootfs()
    }

    /// Checks if the builder container exists using podman images.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if the builder container exists
    /// * `Ok(false)` if the builder container does not exist
    /// * `Err` if the podman command fails
    pub fn check_builder_container_exists(&self) -> Result<bool> {
        let args = vec![
            "--filter".to_string(),
            format!("reference=localhost/{}", self.config.builder_tag),
            "--format".to_string(),
            "{{.Repository}}:{{.Tag}}".to_string(),
        ];

        let output = self
            .executor
            .podman_images(&args)
            .context("Failed to check if builder container exists")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Podman images command failed with exit code: {:?}",
                output.status.code()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected_prefix = format!("localhost/{}:", self.config.builder_tag);

        Ok(stdout
            .lines()
            .any(|line| line.trim().starts_with(&expected_prefix)))
    }
}
