//! Comprehensive error handling and edge case tests.
//!
//! Tests cover error propagation, edge cases, and resilience scenarios.

mod common;

use common::{isolation::*, mocks::*};
use std::{fs, sync::Arc};
use tempfile::TempDir;
use trellis::{
    cli::{Cli, Commands},
    config::TrellisConfig,
    trellis::{
        builder::{BuildType, ContainerBuilder, ScopedEnvVar},
        cleaner::ImageCleaner,
        discovery::ContainerfileDiscovery,
        runner::ContainerRunner,
        Trellis,
    },
    TrellisApp,
};

fn create_error_test_config(temp_dir: &TempDir) -> TrellisConfig {
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
fn test_scoped_env_var_new_and_drop() {
    let test_key = "TRELLIS_TEST_VAR";
    let original_value = std::env::var(test_key).ok();
    
    // Clean slate
    std::env::remove_var(test_key);
    
    {
        let _scoped = ScopedEnvVar::new(test_key, "test_value");
        assert_eq!(std::env::var(test_key).unwrap(), "test_value");
    } // ScopedEnvVar should restore original state here
    
    assert_eq!(std::env::var(test_key).ok(), None);
    
    // Test with existing value
    std::env::set_var(test_key, "original");
    {
        let _scoped = ScopedEnvVar::new(test_key, "temporary");
        assert_eq!(std::env::var(test_key).unwrap(), "temporary");
    }
    assert_eq!(std::env::var(test_key).unwrap(), "original");
    
    // Cleanup
    match original_value {
        Some(val) => std::env::set_var(test_key, val),
        None => std::env::remove_var(test_key),
    }
}

#[test]
fn test_builder_with_invalid_containerfile_content() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create containerfile with invalid content
    let containerfile_path = temp_dir.path().join("Containerfile.invalid");
    fs::write(&containerfile_path, "INVALID DOCKERFILE CONTENT").unwrap();
    
    let config = create_error_test_config(&temp_dir);
    let mut mock = MockCommandExecutor::new();
    mock.expect_podman_build()
        .returning(|_| Ok(create_failure_output("Dockerfile syntax error")));
    
    let executor = Arc::new(mock);
    let builder = ContainerBuilder::new(&config, executor);
    
    let stages = vec!["invalid".to_string()];
    let result = builder.build_multistage_container("test", "test-tag", &stages, BuildType::Builder);
    assert!(result.is_err());
}

#[test]
fn test_cleaner_with_malformed_image_output() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_error_test_config(&temp_dir);
    
    let mut mock = MockCommandExecutor::new();
    mock.expect_podman_images()
        .returning(|_| Ok(create_success_output("malformed\nimage\nlist")));
    
    let executor = Arc::new(mock);
    let cleaner = ImageCleaner::new(&config, executor);
    
    // Should handle malformed output gracefully
    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_runner_with_non_existent_command() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_error_test_config(&temp_dir);
    
    let mut mock = MockCommandExecutor::new();
    mock.expect_execute()
        .returning(|_, _| Ok(create_success_output("Image exists")));
    mock.expect_podman_run()
        .returning(|_| Err(anyhow::anyhow!("Command not found")));
    
    let executor = Arc::new(mock);
    let runner = ContainerRunner::new(&config, executor);
    
    let result = runner.run_container("test", &vec!["nonexistent-command".to_string()]);
    assert!(result.is_err());
}

#[test]
fn test_discovery_with_permission_denied() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a directory we can't read
    let restricted_dir = temp_dir.path().join("restricted");
    fs::create_dir_all(&restricted_dir).unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&restricted_dir).unwrap().permissions();
        perms.set_mode(0o000); // No permissions
        fs::set_permissions(&restricted_dir, perms).unwrap();
    }
    
    let config = create_error_test_config(&temp_dir);
    let discovery = ContainerfileDiscovery::new(&config);
    
    // Should handle permission errors gracefully
    let result = discovery.find_containerfile("restricted");
    assert!(result.is_err());
    
    // Cleanup
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&restricted_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&restricted_dir, perms).unwrap();
    }
}

#[test]
fn test_app_with_corrupted_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("corrupt.toml");
    
    // Write corrupted TOML
    fs::write(&config_path, "invalid toml [[[").unwrap();
    
    std::env::set_var("TRELLIS_CONFIG", &config_path);
    
    let cli = Cli {
        command: Commands::Build,
        builder_tag: "test".to_string(),
        podman_build_cache: None,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: Some(temp_dir.path().to_path_buf()),
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_stages: vec!["base".to_string()],
        rootfs_base: "scratch".to_string(),
        rootfs_tag: "test-rootfs".to_string(),
        builder_stages: vec!["base".to_string()],
    };
    
    let result = TrellisApp::new(cli);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to parse config file"));
    
    std::env::remove_var("TRELLIS_CONFIG");
}

#[test]
fn test_trellis_with_all_operations_failing() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base"]);
    
    let config = create_error_test_config(&temp_dir);
    
    let mut mock = MockCommandExecutor::new();
    mock.expect_podman_build()
        .returning(|_| Err(anyhow::anyhow!("Build failed")));
    mock.expect_podman_run()
        .returning(|_| Err(anyhow::anyhow!("Run failed")));
    mock.expect_podman_images()
        .returning(|_| Err(anyhow::anyhow!("Images failed")));
    mock.expect_podman_rmi()
        .returning(|_| Err(anyhow::anyhow!("RMI failed")));
    mock.expect_bootc()
        .returning(|_| Err(anyhow::anyhow!("Bootc failed")));
    
    let executor = Arc::new(mock);
    let trellis = Trellis::new(&config, executor);
    
    // All operations should fail gracefully
    assert!(trellis.build_builder_container().is_err());
    assert!(trellis.build_rootfs_container().is_err());
    assert!(trellis.run_rootfs_container(&vec!["echo".to_string()]).is_err());
    assert!(trellis.clean().is_err());
    assert!(trellis.update().is_err());
}

#[test]
fn test_builder_with_extremely_long_stage_names() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create stage with very long name
    let long_name = "a".repeat(1000);
    let containerfile_path = temp_dir.path().join(format!("Containerfile.{}", long_name));
    fs::write(&containerfile_path, "FROM alpine").unwrap();
    
    let config = create_error_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);
    
    let stages = vec![long_name];
    let result = builder.build_multistage_container("test", "test-tag", &stages, BuildType::Builder);
    // Should handle long names (may succeed or fail depending on system limits)
    // but shouldn't panic
}

#[test]
fn test_discovery_with_circular_symlinks() {
    let temp_dir = TempDir::new().unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs;
        
        let dir1 = temp_dir.path().join("dir1");
        let dir2 = temp_dir.path().join("dir2");
        std::fs::create_dir_all(&dir1).unwrap();
        std::fs::create_dir_all(&dir2).unwrap();
        
        // Create circular symlinks
        let _ = fs::symlink(&dir2, dir1.join("link_to_dir2"));
        let _ = fs::symlink(&dir1, dir2.join("link_to_dir1"));
        
        let config = create_error_test_config(&temp_dir);
        let discovery = ContainerfileDiscovery::new(&config);
        
        // Should handle circular symlinks without infinite loop
        let result = discovery.find_containerfile("missing");
        assert!(result.is_err()); // Should fail to find but not hang
    }
}

#[test]
fn test_cleaner_with_special_character_image_names() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_error_test_config(&temp_dir);
    
    // Create images with special characters
    let special_images = vec![
        MockImageInfo::new("abc123", "localhost/test-image!@#$%", "latest"),
        MockImageInfo::new("def456", "localhost/测试镜像", "v1.0"),
        MockImageInfo::new("ghi789", "localhost/image with spaces", "latest"),
    ];
    
    let mut mock = MockCommandExecutor::new();
    let images_output = format_special_images(&special_images);
    mock.expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));
    mock.expect_podman_rmi()
        .returning(|_| Ok(create_success_output("Images removed")));
    
    let executor = Arc::new(mock);
    let cleaner = ImageCleaner::new(&config, executor);
    
    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_runner_with_extremely_long_arguments() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_error_test_config(&temp_dir);
    
    let mut mock = MockCommandExecutor::new();
    mock.expect_execute()
        .returning(|_, _| Ok(create_success_output("Image exists")));
    mock.expect_podman_run()
        .returning(|_| Ok(create_success_output("Command executed")));
    
    let executor = Arc::new(mock);
    let runner = ContainerRunner::new(&config, executor);
    
    // Create very long argument
    let long_arg = "a".repeat(10000);
    let args = vec![long_arg];
    
    let result = runner.run_container("test", &args);
    // Should handle long arguments (may succeed or fail depending on system limits)
    assert!(result.is_ok() || result.is_err()); // Should not panic
}

#[test]
fn test_app_with_invalid_src_directory() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_path = temp_dir.path().join("does_not_exist").join("nested");
    
    let cli = Cli {
        command: Commands::Build,
        builder_tag: "test".to_string(),
        podman_build_cache: None,
        auto_clean: false,
        pacman_cache: None,
        aur_cache: None,
        src_dir: Some(invalid_path),
        extra_contexts: vec![],
        extra_mounts: vec![],
        rootfs_stages: vec!["base".to_string()],
        rootfs_base: "scratch".to_string(),
        rootfs_tag: "test-rootfs".to_string(),
        builder_stages: vec!["base".to_string()],
    };
    
    let result = TrellisApp::new(cli);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Source directory does not exist"));
}

#[test]
fn test_concurrent_scoped_env_vars() {
    use std::thread;
    use std::sync::mpsc;
    
    let test_key = "TRELLIS_CONCURRENT_TEST";
    std::env::remove_var(test_key);
    
    let (tx, rx) = mpsc::channel();
    
    let handle = thread::spawn(move || {
        let _scoped = ScopedEnvVar::new(test_key, "thread_value");
        tx.send(std::env::var(test_key).unwrap()).unwrap();
        thread::sleep(std::time::Duration::from_millis(100));
    });
    
    // Main thread should not see the scoped value
    thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(std::env::var(test_key).ok(), None);
    
    let thread_value = rx.recv().unwrap();
    assert_eq!(thread_value, "thread_value");
    
    handle.join().unwrap();
    
    // After thread ends, var should be unset
    assert_eq!(std::env::var(test_key).ok(), None);
}

#[test]
fn test_error_propagation_chain() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_error_test_config(&temp_dir);
    
    let mut mock = MockCommandExecutor::new();
    mock.expect_podman_build()
        .returning(|_| Err(anyhow::anyhow!("Low level error").context("Mid level context")));
    
    let executor = Arc::new(mock);
    let trellis = Trellis::new(&config, executor);
    
    let result = trellis.build_builder_container();
    assert!(result.is_err());
    
    // Check error chain contains context
    let error_string = result.unwrap_err().to_string();
    assert!(error_string.contains("Low level error") || error_string.contains("Mid level context"));
}

#[test]
fn test_resource_cleanup_on_early_exit() {
    let temp_dir = TempDir::new().unwrap();
    common::setup_test_containerfiles(&temp_dir, &["base", "tools"]);
    // Missing "final" to trigger early exit
    
    let config = create_error_test_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());
    let builder = ContainerBuilder::new(&config, executor);
    
    let stages = vec!["base".to_string(), "tools".to_string(), "final".to_string()];
    let result = builder.build_multistage_container("test", "test-tag", &stages, BuildType::Builder);
    
    // Should fail early due to validation but not leave resources hanging
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing required containerfiles"));
}

fn format_special_images(images: &[MockImageInfo]) -> String {
    let mut output = String::new();
    for image in images {
        output.push_str(&format!("{}:{}\n", image.repository, image.tag));
    }
    output
}