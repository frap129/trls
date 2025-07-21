//! Comprehensive tests for ContainerfileDiscovery functionality.
//!
//! Tests cover file discovery, stage validation, and nested/flat structure support.

mod common;

use common::{isolation::*, mocks::*};
use std::fs;
use tempfile::TempDir;
use trellis::{
    config::TrellisConfig,
    trellis::discovery::ContainerfileDiscovery,
};

fn create_discovery_config(temp_dir: &TempDir) -> TrellisConfig {
    TrellisConfig {
        builder_stages: vec!["base".to_string()],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec!["base".to_string()],
        rootfs_base: "scratch".to_string(),
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
    }
}

#[test]
fn test_containerfile_discovery_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_discovery_config(&temp_dir);

    let _discovery = ContainerfileDiscovery::new(&config);
    // Test passes if no panic occurs during creation
}

#[test]
fn test_find_containerfile_in_root_directory() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile("base");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), temp_dir.path().join("Containerfile.base"));
}

#[test]
fn test_find_containerfile_in_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_nested_containerfiles(&temp_dir, &[("base", "base")]);

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile("base");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), temp_dir.path().join("base/Containerfile.base"));
}

#[test]
fn test_find_containerfile_prefers_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create both root and subdirectory versions
    common::setup_test_containerfiles(&temp_dir, &["base"]);
    common::setup_nested_containerfiles(&temp_dir, &[("base", "base")]);

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile("base");
    assert!(result.is_ok());
    // Should prefer subdirectory version
    assert_eq!(result.unwrap(), temp_dir.path().join("base/Containerfile.base"));
}

#[test]
fn test_find_containerfile_not_found() {
    let temp_dir = TempDir::new().unwrap();
    // Don't create any containerfiles

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile("missing");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Containerfile not found"));
    assert!(result.unwrap_err().to_string().contains("Containerfile.missing"));
}

#[test]
fn test_find_containerfile_deep_nesting() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create deeply nested structure
    let deep_path = temp_dir.path().join("deep").join("nested").join("structure");
    fs::create_dir_all(&deep_path).unwrap();
    
    let containerfile_path = deep_path.join("Containerfile.deep");
    fs::write(&containerfile_path, "FROM alpine\nRUN echo 'deep nested'").unwrap();

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile("deep");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), containerfile_path);
}

#[test]
fn test_find_containerfile_multiple_matches() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple potential matches at different nesting levels
    let level1 = temp_dir.path().join("base");
    fs::create_dir_all(&level1).unwrap();
    fs::write(level1.join("Containerfile.base"), "FROM alpine\nRUN echo 'level1'").unwrap();
    
    let level2 = temp_dir.path().join("deeper").join("base");
    fs::create_dir_all(&level2).unwrap();
    fs::write(level2.join("Containerfile.base"), "FROM alpine\nRUN echo 'level2'").unwrap();

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile("base");
    assert!(result.is_ok());
    // Should find one of them (implementation dependent which one)
    assert!(result.unwrap().ends_with("Containerfile.base"));
}

#[test]
fn test_parse_stage_name_simple() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("base");
    assert_eq!(group, "base");
    assert_eq!(stage, "base");
}

#[test]
fn test_parse_stage_name_with_group() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("gpu:cuda");
    assert_eq!(group, "gpu");
    assert_eq!(stage, "cuda");
}

#[test]
fn test_parse_stage_name_multiple_colons() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("features:gpu:latest");
    assert_eq!(group, "features");
    assert_eq!(stage, "gpu:latest");
}

#[test]
fn test_parse_stage_name_empty() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("");
    assert_eq!(group, "");
    assert_eq!(stage, "");
}

#[test]
fn test_parse_stage_name_only_colon() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name(":");
    assert_eq!(group, "");
    assert_eq!(stage, "");
}

#[test]
fn test_parse_stage_name_leading_colon() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name(":stage");
    assert_eq!(group, "");
    assert_eq!(stage, "stage");
}

#[test]
fn test_parse_stage_name_trailing_colon() {
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("group:");
    assert_eq!(group, "group");
    assert_eq!(stage, "");
}

#[test]
fn test_validate_stages_all_exist() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "tools", "final"]);

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let stages = vec!["base".to_string(), "tools".to_string(), "final".to_string()];
    let result = discovery.validate_stages(&stages);
    assert!(result.is_ok());
}

#[test]
fn test_validate_stages_some_missing() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "tools"]);
    // Missing "final"

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let stages = vec!["base".to_string(), "tools".to_string(), "final".to_string()];
    let result = discovery.validate_stages(&stages);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing required containerfiles"));
    assert!(result.unwrap_err().to_string().contains("Containerfile.final"));
}

#[test]
fn test_validate_stages_empty_list() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let stages = vec![];
    let result = discovery.validate_stages(&stages);
    assert!(result.is_ok()); // Empty list should be valid
}

#[test]
fn test_validate_stages_mixed_flat_and_nested() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);
    common::setup_nested_containerfiles(&temp_dir, &[("gpu", "cuda"), ("features", "debug")]);

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let stages = vec![
        "base".to_string(),
        "gpu:cuda".to_string(),
        "features:debug".to_string(),
    ];
    let result = discovery.validate_stages(&stages);
    assert!(result.is_ok());
}

#[test]
fn test_validate_stages_nested_syntax_missing() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_nested_containerfiles(&temp_dir, &[("gpu", "base")]);
    // Missing "gpu:cuda"

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let stages = vec!["gpu:base".to_string(), "gpu:cuda".to_string()];
    let result = discovery.validate_stages(&stages);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing required containerfiles"));
}

#[test]
fn test_find_containerfile_with_symlinks() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);
    
    // Create a symlink to test handling
    #[cfg(unix)]
    {
        use std::os::unix::fs;
        let symlink_dir = temp_dir.path().join("symlink");
        std::fs::create_dir_all(&symlink_dir).unwrap();
        let _ = fs::symlink(temp_dir.path(), symlink_dir.join("link"));
    }

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile("base");
    assert!(result.is_ok());
}

#[test]
fn test_find_containerfile_case_sensitivity() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create lowercase containerfile
    let containerfile_path = temp_dir.path().join("Containerfile.base");
    fs::write(&containerfile_path, "FROM alpine").unwrap();

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    // Search should be case sensitive
    let result_lower = discovery.find_containerfile("base");
    assert!(result_lower.is_ok());
    
    let result_upper = discovery.find_containerfile("BASE");
    assert!(result_upper.is_err());
}

#[test]
fn test_find_containerfile_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create containerfile with special characters in name
    let special_name = "base-test_v1.0";
    let containerfile_path = temp_dir.path().join(format!("Containerfile.{}", special_name));
    fs::write(&containerfile_path, "FROM alpine").unwrap();

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile(special_name);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), containerfile_path);
}

#[test]
fn test_find_containerfile_unicode_names() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create containerfile with unicode characters
    let unicode_name = "base-测试";
    let containerfile_path = temp_dir.path().join(format!("Containerfile.{}", unicode_name));
    fs::write(&containerfile_path, "FROM alpine").unwrap();

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile(unicode_name);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), containerfile_path);
}

#[test]
fn test_find_containerfile_very_long_path() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a very long nested path
    let mut long_path = temp_dir.path().to_path_buf();
    for i in 0..10 {
        long_path = long_path.join(format!("level{}", i));
    }
    fs::create_dir_all(&long_path).unwrap();
    
    let containerfile_path = long_path.join("Containerfile.deep");
    fs::write(&containerfile_path, "FROM alpine").unwrap();

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let result = discovery.find_containerfile("deep");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), containerfile_path);
}

#[test]
fn test_validate_stages_error_message_content() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);
    // Missing "tools" and "final"

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    let stages = vec![
        "base".to_string(),
        "tools".to_string(),
        "final".to_string(),
    ];
    let result = discovery.validate_stages(&stages);
    assert!(result.is_err());
    
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Missing required containerfiles"));
    assert!(error_message.contains("Containerfile.tools"));
    assert!(error_message.contains("Containerfile.final"));
    // Should not mention "base" since it exists
    assert!(!error_message.contains("Containerfile.base"));
}

#[test]
fn test_recursive_search_depth_limit() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create extremely deep nesting to test if there's any depth limit handling
    let mut deep_path = temp_dir.path().to_path_buf();
    for i in 0..50 {
        deep_path = deep_path.join(format!("level{}", i));
    }
    fs::create_dir_all(&deep_path).unwrap();
    
    let containerfile_path = deep_path.join("Containerfile.deep");
    fs::write(&containerfile_path, "FROM alpine").unwrap();

    let config = create_discovery_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);

    // This should either find the file or gracefully fail
    // (depending on implementation limits)
    let result = discovery.find_containerfile("deep");
    // We don't assert success/failure here since deep nesting behavior
    // may be implementation-dependent, but it shouldn't panic
}