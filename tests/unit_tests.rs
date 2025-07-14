use std::fs;
use tempfile::TempDir;

use trellis::{
    cli::{Cli, Commands},
    config::{Config, TrellisConfig},
    TrellisApp, ContainerfileDiscovery,
};

fn create_test_cli() -> Cli {
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

fn create_test_config(temp_dir: &TempDir) -> TrellisConfig {
    TrellisConfig {
        builder_stages: vec!["base".to_string()],
        builder_tag: "test-builder".to_string(),
        podman_build_cache: false,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: temp_dir.path().to_path_buf(),
        rootfs_stages: vec!["base".to_string(), "final".to_string()],
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_tag: "test-rootfs".to_string(),
        hooks_dir: None,
    }
}

// Config tests
#[test]
fn test_config_default() {
    let config = Config::default();
    assert!(config.build.is_some());
    assert!(config.environment.is_some());
    
    let build = config.build.unwrap();
    assert_eq!(build.builder_tag, Some("trellis-builder".to_string()));
    assert_eq!(build.rootfs_tag, Some("trellis-rootfs".to_string()));
    assert_eq!(build.podman_build_cache, Some(false));
}

#[test]
fn test_trellis_config_with_defaults() {
    let cli = create_test_cli();
    let config = TrellisConfig::new(cli).unwrap();
    
    // CLI values should override defaults when they differ from defaults
    assert_eq!(config.builder_tag, "test-builder");
    assert_eq!(config.rootfs_tag, "test-rootfs");
    assert!(!config.podman_build_cache);
    
    // Note: If system config file exists, it will provide default stages
    // This test validates that CLI overrides work even with system config
    // The actual stages depend on whether /etc/trellis/trellis.toml exists
}

#[test]
fn test_trellis_config_with_cli_overrides() {
    let mut cli = create_test_cli();
    cli.builder_stages = vec!["stage1".to_string(), "stage2".to_string()];
    cli.rootfs_stages = vec!["base".to_string(), "final".to_string()];
    cli.podman_build_cache = Some(true);
    
    let config = TrellisConfig::new(cli).unwrap();
    
    assert_eq!(config.builder_stages, vec!["stage1", "stage2"]);
    assert_eq!(config.rootfs_stages, vec!["base", "final"]);
    assert!(config.podman_build_cache);
}

#[test]
fn test_trellis_config_with_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("trellis.toml");
    
    let config_content = r#"
[build]
builder_stages = ["builder1", "builder2"]
rootfs_stages = ["base", "apps"]
builder_tag = "file-builder"
rootfs_tag = "file-rootfs"
podman_build_cache = true

[environment]
src_dir = "/custom/src"
pacman_cache = "/custom/pacman"
aur_cache = "/custom/aur"
hooks_dir = "/custom/hooks"
"#;
    
    fs::write(&config_path, config_content).unwrap();
    
    // Note: This test would need modification to support custom config paths
    // For now, it demonstrates the config parsing logic
    let parsed: Config = toml::from_str(config_content).unwrap();
    
    assert!(parsed.build.is_some());
    let build = parsed.build.unwrap();
    assert_eq!(build.builder_stages, Some(vec!["builder1".to_string(), "builder2".to_string()]));
    assert_eq!(build.rootfs_stages, Some(vec!["base".to_string(), "apps".to_string()]));
    assert_eq!(build.builder_tag, Some("file-builder".to_string()));
    assert_eq!(build.podman_build_cache, Some(true));
}

// Discovery tests
#[test]
fn test_find_containerfile_in_subdir() {
    let temp_dir = TempDir::new().unwrap();
    let base_dir = temp_dir.path().join("base");
    fs::create_dir_all(&base_dir).unwrap();
    
    let containerfile_path = base_dir.join("Containerfile.base");
    fs::write(&containerfile_path, "FROM alpine").unwrap();
    
    let config = create_test_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);
    
    let result = discovery.find_containerfile("base").unwrap();
    assert_eq!(result, containerfile_path.to_string_lossy());
}

#[test]
fn test_find_containerfile_in_root() {
    let temp_dir = TempDir::new().unwrap();
    let containerfile_path = temp_dir.path().join("Containerfile.base");
    fs::write(&containerfile_path, "FROM alpine").unwrap();
    
    let config = create_test_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);
    
    let result = discovery.find_containerfile("base").unwrap();
    assert_eq!(result, containerfile_path.to_string_lossy());
}

#[test]
fn test_find_containerfile_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);
    
    let result = discovery.find_containerfile("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Containerfile not found"));
}

#[test]
fn test_find_containerfile_prefers_subdir() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create both versions
    let root_containerfile = temp_dir.path().join("Containerfile.base");
    fs::write(&root_containerfile, "FROM alpine:root").unwrap();
    
    let base_dir = temp_dir.path().join("base");
    fs::create_dir_all(&base_dir).unwrap();
    let subdir_containerfile = base_dir.join("Containerfile.base");
    fs::write(&subdir_containerfile, "FROM alpine:subdir").unwrap();
    
    let config = create_test_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);
    
    let result = discovery.find_containerfile("base").unwrap();
    assert_eq!(result, subdir_containerfile.to_string_lossy());
}

#[test]
fn test_find_containerfile_recursive_search() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a deeply nested directory structure
    let nested_dir = temp_dir.path().join("deeply").join("nested").join("subdir");
    fs::create_dir_all(&nested_dir).unwrap();
    
    // Place a Containerfile in the nested directory
    let containerfile_path = nested_dir.join("Containerfile.root");
    fs::write(&containerfile_path, "FROM alpine:recursive").unwrap();
    
    let config = create_test_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);
    
    // The recursive search should find the deeply nested Containerfile
    let result = discovery.find_containerfile("root").unwrap();
    assert_eq!(result, containerfile_path.to_string_lossy());
}

#[test]
fn test_trellis_app_creation() {
    let cli = Cli {
        command: Commands::Build,
        builder_tag: "test-builder".to_string(),
        podman_build_cache: None,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: None,
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_stages: vec!["base".to_string()],
        rootfs_tag: "test-rootfs".to_string(),
        builder_stages: vec![],
    };
    
    let app = TrellisApp::new(cli);
    assert!(app.is_ok());
}

#[test]
fn test_auto_clean_config() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test CLI override of auto-clean
    let cli = Cli {
        command: Commands::Build,
        builder_tag: "test-builder".to_string(),
        podman_build_cache: None,
        auto_clean: true,
        pacman_cache: None,
        aur_cache: None,
        src_dir: Some(temp_dir.path().to_path_buf()),
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_stages: vec!["base".to_string()],
        rootfs_tag: "test-rootfs".to_string(),
        builder_stages: vec![],
    };
    
    let config = TrellisConfig::new(cli).unwrap();
    assert!(config.auto_clean);
}

#[test]
fn test_build_rootfs_container_no_stages() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_test_config(&temp_dir);
    config.rootfs_stages = vec![];
    
    let trellis = trellis::Trellis::new(&config);
    let result = trellis.build_rootfs_container();
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No rootfs stages defined"));
}

#[test]
fn test_build_builder_container_no_stages() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_test_config(&temp_dir);
    config.builder_stages = vec![];
    
    let trellis = trellis::Trellis::new(&config);
    let result = trellis.build_builder_container();
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No builder stages defined"));
}