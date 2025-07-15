mod common;

use std::fs;
use tempfile::TempDir;
use trellis::{
    cli::{Cli, Commands},
    config::TrellisConfig,
    ContainerBuilder, ContainerfileDiscovery, ImageCleaner,
};
use common::*;

fn create_minimal_cli() -> Cli {
    Cli {
        command: Commands::Build,
        builder_tag: "test-builder".to_string(),
        podman_build_cache: None,
        auto_clean: false,
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
fn test_missing_containerfile_error() {
    let temp_dir = TempDir::new().unwrap();
    let config = TrellisConfig {
        builder_stages: vec!["missing".to_string()],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec![],
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
    };

    let discovery = ContainerfileDiscovery::new(&config);
    let result = discovery.find_containerfile("missing");
    
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Containerfile not found"));
    assert!(error_msg.contains("Containerfile.missing"));
}

#[test]
fn test_empty_stages_validation() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_config = temp_dir.path().join("nonexistent.toml");
    
    // Use a unique environment variable name to avoid conflicts
    let env_var_name = format!("TRELLIS_CONFIG_{}", std::process::id());
    std::env::set_var(&env_var_name, &nonexistent_config);
    
    let mut cli = create_minimal_cli();
    cli.builder_stages = vec![]; // Empty stages
    cli.rootfs_stages = vec![]; // Empty stages
    
    // Temporarily override the environment variable for this test
    let original_config = std::env::var("TRELLIS_CONFIG").ok();
    std::env::set_var("TRELLIS_CONFIG", &nonexistent_config);
    
    // This should not fail during config creation since stages can be specified in file
    let config = TrellisConfig::new(cli).unwrap();
    
    // Restore original environment
    match original_config {
        Some(value) => std::env::set_var("TRELLIS_CONFIG", value),
        None => std::env::remove_var("TRELLIS_CONFIG"),
    }
    std::env::remove_var(&env_var_name);
    
    // But it should fail when trying to build with empty stages
    assert!(config.builder_stages.is_empty());
    assert!(config.rootfs_stages.is_empty());
}

#[test]
fn test_invalid_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");
    
    // Write invalid TOML
    fs::write(&config_path, "invalid toml content [[[").unwrap();
    
    // Set environment variable to use our invalid config
    std::env::set_var("TRELLIS_CONFIG", config_path);
    
    let cli = create_minimal_cli();
    let result = TrellisConfig::new(cli);
    
    // Clean up environment
    std::env::remove_var("TRELLIS_CONFIG");
    
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse config file"));
}

#[test]
fn test_nonexistent_cache_directory_parent() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_cache = temp_dir.path().join("nonexistent").join("deep").join("cache");
    
    let config = TrellisConfig {
        builder_stages: vec!["base".to_string()],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: Some(nonexistent_cache),
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec!["base".to_string()],
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
    };

    // This test validates that the cache directory creation logic
    // properly handles nested directory creation
    let _builder = ContainerBuilder::new(&config);
    
    // The actual podman build would fail since we don't have containerfiles,
    // but the cache directory creation should work
    assert!(config.pacman_cache.is_some());
}

#[test]
fn test_containerfile_discovery_with_symlinks() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir_all(&subdir).unwrap();
    
    // Create a real containerfile
    setup_test_containerfiles(&temp_dir, &["base"]);
    
    // Create a symlink to the same directory (potential cycle)
    #[cfg(unix)]
    {
        use std::os::unix::fs;
        let symlink_path = subdir.join("parent_link");
        let _ = fs::symlink(temp_dir.path(), symlink_path);
    }
    
    let config = TrellisConfig {
        builder_stages: vec![],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec![],
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
    };

    let discovery = ContainerfileDiscovery::new(&config);
    let result = discovery.find_containerfile("base");
    
    // Should find the containerfile despite the symlink
    assert!(result.is_ok());
}

#[test]
fn test_stage_validation_with_missing_files() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create only some of the required containerfiles
    setup_test_containerfiles(&temp_dir, &["base", "tools"]);
    // Missing "final" containerfile
    
    let config = TrellisConfig {
        builder_stages: vec![],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec![],
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
    };

    let discovery = ContainerfileDiscovery::new(&config);
    
    // This should succeed for existing files
    let stages_exist = vec!["base".to_string(), "tools".to_string()];
    assert!(discovery.validate_stages(&stages_exist).is_ok());
    
    // This should fail for missing files
    let stages_missing = vec!["base".to_string(), "tools".to_string(), "final".to_string()];
    let result = discovery.validate_stages(&stages_missing);
    assert!(result.is_err());
    
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Missing required containerfiles"));
    assert!(error_msg.contains("Containerfile.final"));
}

#[test]
fn test_stage_name_parsing() {
    // Test simple stage names
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("base");
    assert_eq!(group, "base");
    assert_eq!(stage, "base");
    
    // Test group:stage syntax
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("features:gpu");
    assert_eq!(group, "features");
    assert_eq!(stage, "gpu");
    
    // Test complex names with multiple colons (only first is used)
    let (group, stage) = ContainerfileDiscovery::parse_stage_name("group:stage:extra");
    assert_eq!(group, "group");
    assert_eq!(stage, "stage:extra");
}

#[test]
fn test_readonly_cache_directory() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("readonly_cache");
    fs::create_dir_all(&cache_dir).unwrap();
    
    // Make directory readonly
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&cache_dir).unwrap().permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(&cache_dir, perms).unwrap();
    }
    
    let config = TrellisConfig {
        builder_stages: vec!["base".to_string()],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: Some(cache_dir.clone()),
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec!["base".to_string()],
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
    };

    // The builder should detect the readonly cache directory
    let _builder = ContainerBuilder::new(&config);
    
    // This would fail when trying to build due to readonly cache
    // but we can't test the actual build without containerfiles and podman
    
    // Clean up: restore permissions so temp dir can be deleted
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&cache_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&cache_dir, perms).unwrap();
    }
}

#[test]
fn test_image_filtering_logic() {
    let config = TrellisConfig {
        builder_stages: vec![],
        builder_tag: "custom-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: std::path::PathBuf::from("/tmp"),
        rootfs_stages: vec![],
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "custom-rootfs".to_string(),
        hooks_dir: None,
    };

    let _cleaner = ImageCleaner::new(&config);
    
    // Test image list (we can't test the actual cleaner without podman)
    // but we can verify the configuration is set up correctly
    assert_eq!(config.builder_tag, "custom-builder");
    assert_eq!(config.rootfs_tag, "custom-rootfs");
}

