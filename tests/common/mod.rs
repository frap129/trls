use std::fs;
use std::path::Path;
use tempfile::TempDir;
use trellis::config::TrellisConfig;

pub mod isolation;
pub mod mocks;

#[allow(dead_code)]
pub fn setup_test_containerfiles(temp_dir: &TempDir, stages: &[&str]) {
    for stage in stages {
        let containerfile_content = format!(
            r#"FROM alpine
RUN echo "Building stage: {stage}"
LABEL stage="{stage}"
"#
        );

        let containerfile_path = temp_dir.path().join(format!("Containerfile.{stage}"));
        fs::write(containerfile_path, containerfile_content).unwrap();
    }
}

// Allow dead_code: Used across multiple test files via wildcard imports but not detected by rustc
#[allow(dead_code)]
pub fn setup_nested_containerfiles(temp_dir: &TempDir, groups_and_stages: &[(&str, &str)]) {
    for (group, stage) in groups_and_stages {
        let group_dir = temp_dir.path().join(group);
        fs::create_dir_all(&group_dir).unwrap();

        let containerfile_content = format!(
            r#"FROM alpine
RUN echo "Building group: {group} stage: {stage}"
LABEL group="{group}" stage="{stage}"
"#
        );

        let containerfile_path = group_dir.join(format!("Containerfile.{group}"));
        fs::write(containerfile_path, containerfile_content).unwrap();
    }
}

// Allow dead_code: Used in config_tests.rs via wildcard import but not detected by rustc
#[allow(dead_code)]
pub fn setup_config_file(temp_dir: &TempDir, config_name: &str) -> std::path::PathBuf {
    let config_content = r#"
[build]
builder_stages = ["builder"]
rootfs_stages = ["base", "tools", "final"]
builder_tag = "test-builder"
rootfs_tag = "test-rootfs"
podman_build_cache = true

[environment]
pacman_cache = "/tmp/test-pacman"
aur_cache = "/tmp/test-aur"
"#;

    let config_path = temp_dir.path().join(config_name);
    fs::write(&config_path, config_content).unwrap();
    config_path
}

// Allow dead_code: Used in config_tests.rs via wildcard import but not detected by rustc
#[allow(dead_code)]
pub fn create_test_hooks_dir(temp_dir: &TempDir) -> std::path::PathBuf {
    let hooks_dir = temp_dir.path().join("hooks.d");
    fs::create_dir_all(&hooks_dir).unwrap();

    let hook_content = r#"#!/bin/bash
echo "Test hook executed"
"#;

    let hook_path = hooks_dir.join("test-hook.sh");
    fs::write(&hook_path, hook_content).unwrap();

    // Make the hook executable (on Unix systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms).unwrap();
    }

    hooks_dir
}

// Allow dead_code: Used extensively in config_tests.rs via wildcard import but not detected by rustc
#[allow(dead_code)]
pub fn assert_file_exists(path: &Path) {
    assert!(path.exists(), "File should exist: {}", path.display());
}

// Allow dead_code: Used in config_tests.rs via wildcard import but not detected by rustc
#[allow(dead_code)]
pub fn assert_file_contains(path: &Path, content: &str) {
    let file_content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read file: {}", path.display()));
    assert!(
        file_content.contains(content),
        "File {} should contain: {}",
        path.display(),
        content
    );
}

/// Creates a standard test configuration with default settings
pub fn create_standard_config(temp_dir: &TempDir) -> TrellisConfig {
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

/// Creates a quiet mode test configuration
pub fn create_quiet_config(temp_dir: &TempDir) -> TrellisConfig {
    let mut config = create_standard_config(temp_dir);
    config.quiet = true;
    config
}

/// Test parameter configuration for flag-based variations
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct TestVariation {
    pub quiet: bool,
    pub auto_clean: bool,
    pub podman_build_cache: bool,
}

#[allow(dead_code)]
impl TestVariation {
    pub fn standard() -> Self {
        Self {
            quiet: false,
            auto_clean: false,
            podman_build_cache: false,
        }
    }

    pub fn quiet() -> Self {
        Self {
            quiet: true,
            auto_clean: false,
            podman_build_cache: false,
        }
    }

    pub fn apply_to_config(&self, config: &mut TrellisConfig) {
        config.quiet = self.quiet;
        config.auto_clean = self.auto_clean;
        config.podman_build_cache = self.podman_build_cache;
    }
}

