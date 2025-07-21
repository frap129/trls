//! Comprehensive tests for TrellisApp functionality.
//!
//! Tests cover command dispatch, configuration integration, and error handling.

mod common;

use common::{isolation::*, mocks::*};
use std::sync::Arc;
use tempfile::TempDir;
use trellis::{
    cli::{Cli, Commands},
    config::TrellisConfig,
    TrellisApp,
};

fn create_test_cli_with_command(command: Commands) -> Cli {
    Cli {
        command,
        builder_tag: "test-builder".to_string(),
        podman_build_cache: None,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: None,
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_stages: vec!["base".to_string()],
        rootfs_base: "scratch".to_string(),
        rootfs_tag: "test-rootfs".to_string(),
        builder_stages: vec!["base".to_string()],
    }
}

#[test]
fn test_trellis_app_creation_success() {
    let cli = create_test_cli_with_command(Commands::Build);
    let app = TrellisApp::new(cli);
    assert!(app.is_ok());
}

#[test]
fn test_trellis_app_with_custom_executor() {
    let cli = create_test_cli_with_command(Commands::Build);
    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor);
    assert!(app.is_ok());
}

#[test]
fn test_build_command_execution() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}

#[test]
fn test_build_builder_command_execution() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cli = create_test_cli_with_command(Commands::BuildBuilder);
    cli.src_dir = Some(temp_dir.path().to_path_buf());

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}

#[test]
fn test_run_command_execution() {
    let temp_dir = TempDir::new().unwrap();

    let mut cli = create_test_cli_with_command(Commands::Run {
        args: vec!["echo".to_string(), "hello".to_string()],
    });
    cli.src_dir = Some(temp_dir.path().to_path_buf());

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}

#[test]
fn test_clean_command_execution() {
    let temp_dir = TempDir::new().unwrap();

    let mut cli = create_test_cli_with_command(Commands::Clean);
    cli.src_dir = Some(temp_dir.path().to_path_buf());

    let executor = Arc::new(MockScenarios::multiple_images());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}

#[test]
fn test_update_command_execution() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cli = create_test_cli_with_command(Commands::Update);
    cli.src_dir = Some(temp_dir.path().to_path_buf());

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}

#[test]
fn test_build_with_empty_stages_fails() {
    let temp_dir = TempDir::new().unwrap();

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());
    cli.rootfs_stages = vec![]; // Empty stages should fail

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No rootfs stages"));
}

#[test]
fn test_build_builder_with_empty_stages_fails() {
    let temp_dir = TempDir::new().unwrap();

    let mut cli = create_test_cli_with_command(Commands::BuildBuilder);
    cli.src_dir = Some(temp_dir.path().to_path_buf());
    cli.builder_stages = vec![]; // Empty stages should fail

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No builder stages"));
}

#[test]
fn test_build_with_command_failure() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());

    let executor = Arc::new(MockScenarios::build_failures());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_err());
}

#[test]
fn test_build_with_missing_containerfiles() {
    let temp_dir = TempDir::new().unwrap();
    // Don't create any containerfiles

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Containerfile not found"));
}

#[test]
fn test_configuration_validation_integration() {
    let temp_dir = TempDir::new().unwrap();

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());
    cli.builder_tag = "test-rootfs".to_string(); // Same as rootfs_tag - should fail validation
    cli.rootfs_tag = "test-rootfs".to_string();

    let executor = Arc::new(MockScenarios::all_success());
    let result = TrellisApp::with_executor(cli, executor);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("builder_tag and rootfs_tag cannot be the same"));
}

#[test]
fn test_multi_stage_build_success() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "tools", "final"]);

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());
    cli.rootfs_stages = vec!["base".to_string(), "tools".to_string(), "final".to_string()];

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}

#[test]
fn test_auto_clean_integration() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());
    cli.auto_clean = true;

    let executor = Arc::new(MockScenarios::multiple_images());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}

#[test]
fn test_cache_configuration_applied() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let pacman_cache = temp_dir.path().join("pacman");
    let aur_cache = temp_dir.path().join("aur");

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());
    cli.pacman_cache = Some(pacman_cache.clone());
    cli.aur_cache = Some(aur_cache.clone());

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}

#[test]
fn test_extra_contexts_and_mounts() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());
    cli.extra_contexts = vec!["context1=/tmp".to_string()];
    cli.extra_mounts = vec!["mount1=/opt".to_string().into()];

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}

#[test]
fn test_custom_rootfs_base() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut cli = create_test_cli_with_command(Commands::Build);
    cli.src_dir = Some(temp_dir.path().to_path_buf());
    cli.rootfs_base = "fedora:39".to_string();

    let executor = Arc::new(MockScenarios::all_success());
    let app = TrellisApp::with_executor(cli, executor).unwrap();

    let result = app.run();
    assert!(result.is_ok());
}