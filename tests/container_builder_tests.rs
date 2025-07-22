//! Comprehensive tests for ContainerBuilder functionality.
//!
//! Tests cover build command generation, multi-stage builds, and container operations.

mod common;

use common::mocks::*;
use std::sync::Arc;
use tempfile::TempDir;
use trellis::{
    config::TrellisConfig,
    trellis::{
        builder::{BuildType, ContainerBuilder},
        discovery::ContainerfileDiscovery,
    },
};

fn create_builder_config(temp_dir: &TempDir) -> TrellisConfig {
    TrellisConfig {
        builder_stages: vec!["base".to_string()],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec!["base".to_string(), "final".to_string()],
        rootfs_base: "scratch".to_string(),
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
        quiet: false,
    }
}

#[test]
fn test_container_builder_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());

    let _builder = ContainerBuilder::new(&config, executor);
    // Test passes if no panic occurs during creation
}

#[test]
fn test_determine_base_image_first_stage_rootfs() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let base_image = builder.determine_base_image(0, BuildType::Rootfs, "");
    assert_eq!(base_image, "scratch");
}

#[test]
fn test_determine_base_image_first_stage_builder() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let base_image = builder.determine_base_image(0, BuildType::Builder, "");
    assert_eq!(base_image, "scratch");
}

#[test]
fn test_determine_base_image_subsequent_stage() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let base_image = builder.determine_base_image(1, BuildType::Rootfs, "trellis-stage-base");
    assert_eq!(base_image, "localhost/trellis-stage-base");
}

#[test]
fn test_determine_base_image_custom_rootfs_base() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_builder_config(&temp_dir);
    config.rootfs_base = "fedora:39".to_string();

    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let base_image = builder.determine_base_image(0, BuildType::Rootfs, "");
    assert_eq!(base_image, "fedora:39");
}

#[test]
fn test_build_multistage_container_success() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "final"]);

    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string(), "final".to_string()];
    let result =
        builder.build_multistage_container("stage", "test-rootfs", &stages, BuildType::Rootfs);
    assert!(result.is_ok());
}

#[test]
fn test_build_single_stage_container() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_missing_containerfile() {
    let temp_dir = TempDir::new().unwrap();
    // Don't create any containerfiles

    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["missing".to_string()];
    let result =
        builder.build_multistage_container("stage", "test-rootfs", &stages, BuildType::Rootfs);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required containerfiles"));
}

#[test]
fn test_build_with_empty_stages() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec![];
    let result =
        builder.build_multistage_container("stage", "test-rootfs", &stages, BuildType::Rootfs);
    assert!(result.is_ok()); // Empty stages should be allowed at builder level
}

#[test]
fn test_build_with_command_failure() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::build_failures());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_err());
}

#[test]
fn test_build_with_multistage_syntax() {
    let temp_dir = TempDir::new().unwrap();
    // Create nested structure for multistage syntax
    common::setup_nested_containerfiles(&temp_dir, &[("gpu", "base"), ("gpu", "cuda")]);

    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["gpu:base".to_string(), "gpu:cuda".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    // Multistage discovery is now fixed - both stages use the same Containerfile.gpu
    assert!(result.is_ok());
}

#[test]
fn test_build_with_cache_disabled() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_builder_config(&temp_dir);
    config.podman_build_cache = false;

    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_cache_enabled() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_builder_config(&temp_dir);
    config.podman_build_cache = true;

    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_extra_contexts() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_builder_config(&temp_dir);
    config.extra_contexts = vec!["context1=/tmp".to_string(), "context2=/opt".to_string()];

    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_extra_mounts() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_builder_config(&temp_dir);
    config.extra_mounts = vec![
        "mount1=/var/cache".to_string().into(),
        "mount2=/var/log".to_string().into(),
    ];

    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_pacman_cache() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_builder_config(&temp_dir);
    config.pacman_cache = Some(temp_dir.path().join("pacman"));

    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_aur_cache() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_builder_config(&temp_dir);
    config.aur_cache = Some(temp_dir.path().join("aur"));

    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_both_caches() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_builder_config(&temp_dir);
    config.pacman_cache = Some(temp_dir.path().join("pacman"));
    config.aur_cache = Some(temp_dir.path().join("aur"));

    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_stage_name_parsing_simple() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("base");
    assert_eq!(group, "base");
    assert_eq!(stage, "base");
}

#[test]
fn test_stage_name_parsing_multistage() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("gpu:cuda");
    assert_eq!(group, "gpu");
    assert_eq!(stage, "cuda");
}

#[test]
fn test_stage_name_parsing_complex() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("features:gpu:latest");
    assert_eq!(group, "features");
    assert_eq!(stage, "gpu:latest");
}

#[test]
fn test_build_stage_tagging_single_stage() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "final-tag", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_build_stage_tagging_multi_stage() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "tools", "final"]);

    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string(), "tools".to_string(), "final".to_string()];
    let result =
        builder.build_multistage_container("stage", "final-tag", &stages, BuildType::Rootfs);
    assert!(result.is_ok());
}

#[test]
fn test_build_stage_tagging_multistage_syntax() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_nested_containerfiles(&temp_dir, &[("gpu", "base"), ("gpu", "cuda")]);

    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["gpu:base".to_string(), "gpu:cuda".to_string()];
    let result =
        builder.build_multistage_container("builder", "final-tag", &stages, BuildType::Builder);
    // Multistage discovery is now fixed - both stages use the same Containerfile.gpu  
    assert!(result.is_ok());
}

#[test]
fn test_build_error_propagation() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let config = create_builder_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_build_streaming()
        .returning(|_| Ok(common::mocks::create_failure_status()));

    let executor = Arc::new(mock_executor);
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result =
        builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Podman build failed"));
}

#[test]
fn test_containerfile_validation_before_build() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "tools"]);
    // Missing "final" containerfile

    let config = create_builder_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string(), "tools".to_string(), "final".to_string()];
    let result =
        builder.build_multistage_container("stage", "test-rootfs", &stages, BuildType::Rootfs);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required containerfiles"));
}

#[test]
fn test_build_with_quiet_mode() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_builder_config(&temp_dir);
    config.quiet = true; // Test the quiet path

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_build() // Should use non-streaming method
        .returning(|_| Ok(create_success_output("Build completed")));

    let executor = Arc::new(mock_executor);
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result = builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_ok());
}

#[test]
fn test_build_with_quiet_mode_error() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let mut config = create_builder_config(&temp_dir);
    config.quiet = true;

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_build()
        .returning(|_| Ok(create_failure_output("Build failed in quiet mode")));

    let executor = Arc::new(mock_executor);
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string()];
    let result = builder.build_multistage_container("builder", "test-builder", &stages, BuildType::Builder);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Podman build failed"));
}

#[test]
fn test_build_with_quiet_mode_multistage() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "final"]);

    let mut config = create_builder_config(&temp_dir);
    config.quiet = true;

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_build()
        .times(2) // Two stages
        .returning(|_| Ok(create_success_output("Build completed")));

    let executor = Arc::new(mock_executor);
    let builder = ContainerBuilder::new(&config, executor);

    let stages = vec!["base".to_string(), "final".to_string()];
    let result = builder.build_multistage_container("rootfs", "test-rootfs", &stages, BuildType::Rootfs);
    assert!(result.is_ok());
}
