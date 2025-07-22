//! Comprehensive tests for ImageCleaner functionality.
//!
//! Tests cover image filtering, cleanup modes, and batch operations.

mod common;

use common::mocks::*;
use std::sync::Arc;
use tempfile::TempDir;
use trellis::{config::TrellisConfig, trellis::cleaner::ImageCleaner};

fn create_cleaner_config(temp_dir: &TempDir) -> TrellisConfig {
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
        quiet: false,
    }
}

#[test]
fn test_image_cleaner_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);
    let executor = Arc::new(MockScenarios::all_success());

    let _cleaner = ImageCleaner::new(&config, executor);
    // Test passes if no panic occurs during creation
}

#[test]
fn test_clean_all_success() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);
    let executor = Arc::new(MockScenarios::multiple_images());
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_clean_all_no_images() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);
    let executor = Arc::new(MockScenarios::no_images());
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_auto_clean_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_cleaner_config(&temp_dir);
    config.auto_clean = false;

    let executor = Arc::new(MockScenarios::multiple_images());
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.auto_clean();
    assert!(result.is_ok());
}

#[test]
fn test_auto_clean_enabled() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_cleaner_config(&temp_dir);
    config.auto_clean = true;

    let executor = Arc::new(MockScenarios::multiple_images());
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.auto_clean();
    assert!(result.is_ok());
}

#[test]
fn test_auto_clean_enabled_no_images() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_cleaner_config(&temp_dir);
    config.auto_clean = true;

    let executor = Arc::new(MockScenarios::no_images());
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.auto_clean();
    assert!(result.is_ok());
}

#[test]
fn test_clean_with_images_command_failure() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(|_| Err(anyhow::anyhow!("Images command failed")));

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_err());
    // After executor changes, the error is wrapped in context
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to list podman images"));
}

#[test]
fn test_clean_with_rmi_command_failure() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    // Return images that should be cleaned
    let images = vec![
        MockImageInfo::new("abc123", "localhost/test-builder", "latest"),
        MockImageInfo::new("def456", "localhost/trellis-stage-base", "latest"),
    ];
    let images_output = format_images_output(&images);

    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    mock_executor
        .expect_podman_rmi()
        .times(1..) // Allow multiple calls
        .returning(|_| Err(anyhow::anyhow!("RMI command failed")));

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_err());
    // After executor changes, the error is wrapped in context
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to remove image"));
}

#[test]
fn test_image_filtering_trellis_images() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    // Create images with trellis prefixes
    let images = vec![
        MockImageInfo::new("abc123", "localhost/test-builder", "latest"),
        MockImageInfo::new("def456", "localhost/test-rootfs", "latest"),
        MockImageInfo::new("ghi789", "localhost/trellis-builder-base", "latest"),
        MockImageInfo::new("jkl012", "localhost/trellis-stage-tools", "latest"),
        MockImageInfo::new("mno345", "docker.io/ubuntu", "latest"), // Non-trellis image
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    mock_executor
        .expect_podman_rmi()
        .returning(|_| Ok(create_success_output("Images removed")));

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_image_filtering_non_trellis_images() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    // Create only non-trellis images
    let images = vec![
        MockImageInfo::new("abc123", "docker.io/ubuntu", "latest"),
        MockImageInfo::new("def456", "quay.io/fedora", "39"),
        MockImageInfo::new("ghi789", "registry.example.com/app", "v1.0"),
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    // Should not call rmi since no trellis images to remove
    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_custom_builder_and_rootfs_tags() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_cleaner_config(&temp_dir);
    config.builder_tag = "custom-builder".to_string();
    config.rootfs_tag = "custom-rootfs".to_string();

    let images = vec![
        MockImageInfo::new("abc123", "localhost/custom-builder", "latest"),
        MockImageInfo::new("def456", "localhost/custom-rootfs", "latest"),
        MockImageInfo::new("ghi789", "localhost/trellis-stage-base", "latest"),
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    mock_executor
        .expect_podman_rmi()
        .returning(|_| Ok(create_success_output("Images removed")));

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_batch_image_removal() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    // Create multiple trellis images
    let images = vec![
        MockImageInfo::new("abc123", "localhost/test-builder", "latest"),
        MockImageInfo::new("def456", "localhost/test-rootfs", "latest"),
        MockImageInfo::new("ghi789", "localhost/trellis-builder-base", "latest"),
        MockImageInfo::new("jkl012", "localhost/trellis-stage-tools", "latest"),
        MockImageInfo::new("mno345", "localhost/trellis-stage-final", "latest"),
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    // Expect batch removal of multiple images
    mock_executor
        .expect_podman_rmi()
        .times(1) // Should be called once for batch operation
        .returning(|_| Ok(create_success_output("Multiple images removed")));

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_clean_mode_full_vs_auto() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_cleaner_config(&temp_dir);
    config.auto_clean = true;

    let images = vec![
        MockImageInfo::new("abc123", "localhost/test-builder", "latest"),
        MockImageInfo::new("def456", "localhost/test-rootfs", "latest"),
        MockImageInfo::new("ghi789", "localhost/trellis-stage-intermediate", "latest"),
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    mock_executor
        .expect_podman_rmi()
        .returning(|_| Ok(create_success_output("Images removed")));

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    // Test auto clean (should preserve final tags in auto mode logic)
    let auto_result = cleaner.auto_clean();
    assert!(auto_result.is_ok());

    // Test full clean
    let full_result = cleaner.clean_all();
    assert!(full_result.is_ok());
}

#[test]
fn test_empty_image_list() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor.expect_podman_images().returning(|_| {
        Ok(create_success_output(
            "REPOSITORY\tTAG\tIMAGE ID\tCREATED\tSIZE\n",
        ))
    });

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_malformed_image_list() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor.expect_podman_images().returning(|_| {
        Ok(create_success_output(
            "malformed\nlines\nwithout:proper:format",
        ))
    });

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok()); // Should handle gracefully
}

#[test]
fn test_images_command_stderr_output() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(|_| Ok(create_failure_output("Images command error")));

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to list images"));
}

fn format_images_output(images: &[MockImageInfo]) -> String {
    let mut output = String::new();
    for image in images {
        output.push_str(&format!("{}:{}\n", image.repository, image.tag));
    }
    output
}

#[test]
fn test_batch_removal_with_fallback_to_individual() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    // Create multiple trellis images
    let images = vec![
        MockImageInfo::new("abc123", "localhost/test-builder", "latest"),
        MockImageInfo::new("def456", "localhost/trellis-stage-base", "latest"),
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    // First call (batch) fails, then individual calls succeed
    let mut call_count = 0;
    mock_executor
        .expect_podman_rmi()
        .times(3) // 1 batch + 2 individual
        .returning(move |args| {
            call_count += 1;
            if call_count == 1 && args.len() > 2 {
                // Batch call fails
                Ok(create_failure_output("Batch removal failed"))
            } else {
                // Individual calls succeed
                Ok(create_success_output("Image removed"))
            }
        });

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_single_image_removal_success() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    // Only one image to remove
    let images = vec![
        MockImageInfo::new("abc123", "localhost/trellis-stage-base", "latest"),
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    mock_executor
        .expect_podman_rmi()
        .times(1)
        .returning(|_| Ok(create_success_output("Image removed")));

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_single_image_removal_failure() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    // Only one image to remove
    let images = vec![
        MockImageInfo::new("abc123", "localhost/trellis-stage-base", "latest"),
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    mock_executor
        .expect_podman_rmi()
        .times(1)
        .returning(|_| Ok(create_failure_output("Image removal failed")));

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok()); // Should handle gracefully and continue
}

#[test]
fn test_batch_removal_command_error_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_cleaner_config(&temp_dir);

    // Create multiple trellis images
    let images = vec![
        MockImageInfo::new("abc123", "localhost/test-builder", "latest"),
        MockImageInfo::new("def456", "localhost/trellis-stage-base", "latest"),
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    // First call (batch) fails with command error, then individual calls succeed
    let mut call_count = 0;
    mock_executor
        .expect_podman_rmi()
        .times(3) // 1 batch + 2 individual
        .returning(move |args| {
            call_count += 1;
            if call_count == 1 && args.len() > 2 {
                // Batch call fails with command error
                Err(anyhow::anyhow!("Command execution failed"))
            } else {
                // Individual calls succeed
                Ok(create_success_output("Image removed"))
            }
        });

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.clean_all();
    assert!(result.is_ok());
}

#[test]
fn test_clean_mode_auto_preserves_final_tags() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_cleaner_config(&temp_dir);
    config.auto_clean = true;

    // Include both final tags and intermediate images
    let images = vec![
        MockImageInfo::new("abc123", "localhost/test-builder", "latest"), // Final tag - should be preserved
        MockImageInfo::new("def456", "localhost/test-rootfs", "latest"),  // Final tag - should be preserved
        MockImageInfo::new("ghi789", "localhost/trellis-stage-intermediate", "latest"), // Should be removed
        MockImageInfo::new("jkl012", "localhost/trellis-builder-temp", "latest"), // Should be removed
    ];
    let images_output = format_images_output(&images);

    let mut mock_executor = MockCommandExecutor::new();
    mock_executor
        .expect_podman_images()
        .returning(move |_| Ok(create_success_output(&images_output)));

    // Should only remove intermediate images, not final builder/rootfs tags
    mock_executor
        .expect_podman_rmi()
        .returning(|args| {
            // In auto mode, should not remove final builder/rootfs images
            for arg in args {
                assert!(!arg.contains("localhost/test-builder:latest"));
                assert!(!arg.contains("localhost/test-rootfs:latest"));
            }
            Ok(create_success_output("Images removed"))
        });

    let executor = Arc::new(mock_executor);
    let cleaner = ImageCleaner::new(&config, executor);

    let result = cleaner.auto_clean();
    assert!(result.is_ok());
}
