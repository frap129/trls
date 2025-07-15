use std::fs;
use std::path::Path;
use tempfile::TempDir;
use assert_cmd::Command;
use trellis::{
    cli::{Cli, Commands},
    config::TrellisConfig,
};

/// Test constants for consistent testing
pub mod test_constants {
    pub const DEFAULT_BUILDER_TAG: &str = "test-builder";
    pub const DEFAULT_ROOTFS_TAG: &str = "test-rootfs";
    pub const DEFAULT_ROOTFS_BASE: &str = "scratch";
    
    pub mod error_messages {
        pub const NO_ROOTFS_STAGES: &str = "No rootfs stages defined";
        pub const NO_BUILDER_STAGES: &str = "No builder stages defined";
        pub const MISSING_CONTAINERFILES: &str = "Missing required containerfiles";
        pub const CONTAINERFILE_NOT_FOUND: &str = "Containerfile not found";
    }
}

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
        
        let containerfile_path = group_dir.join(format!("Containerfile.{stage}"));
        fs::write(containerfile_path, containerfile_content).unwrap();
    }
}

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

pub fn assert_file_exists(path: &Path) {
    assert!(path.exists(), "File should exist: {}", path.display());
}

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

/// Test configuration builder for consistent test setup
pub struct TestConfigBuilder {
    temp_dir: TempDir,
    rootfs_base: String,
    rootfs_stages: Vec<String>,
    builder_stages: Vec<String>,
    builder_tag: String,
    rootfs_tag: String,
}

impl TestConfigBuilder {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
            rootfs_base: test_constants::DEFAULT_ROOTFS_BASE.to_string(),
            rootfs_stages: vec![],
            builder_stages: vec![],
            builder_tag: test_constants::DEFAULT_BUILDER_TAG.to_string(),
            rootfs_tag: test_constants::DEFAULT_ROOTFS_TAG.to_string(),
        }
    }
    
    pub fn with_rootfs_base(mut self, base: &str) -> Self {
        self.rootfs_base = base.to_string();
        self
    }
    
    pub fn with_rootfs_stages(mut self, stages: &[&str]) -> Self {
        self.rootfs_stages = stages.iter().map(|s| s.to_string()).collect();
        self
    }
    
    pub fn with_builder_stages(mut self, stages: &[&str]) -> Self {
        self.builder_stages = stages.iter().map(|s| s.to_string()).collect();
        self
    }
    
    pub fn with_builder_tag(mut self, tag: &str) -> Self {
        self.builder_tag = tag.to_string();
        self
    }
    
    pub fn with_rootfs_tag(mut self, tag: &str) -> Self {
        self.rootfs_tag = tag.to_string();
        self
    }
    
    pub fn build(self) -> TrellisConfig {
        TrellisConfig {
            builder_stages: self.builder_stages,
            builder_tag: self.builder_tag,
            podman_build_cache: false,
            auto_clean: false,
            pacman_cache: None,
            aur_cache: None,
            src_dir: self.temp_dir.path().to_path_buf(),
            rootfs_stages: self.rootfs_stages,
            rootfs_base: self.rootfs_base,
            extra_contexts: vec![],
            extra_mounts: vec![],
            rootfs_tag: self.rootfs_tag,
            hooks_dir: None,
        }
    }
    
    pub fn temp_dir(&self) -> &TempDir {
        &self.temp_dir
    }
}

/// CLI builder for consistent test command setup
pub struct TestCliBuilder {
    rootfs_base: String,
    rootfs_stages: Vec<String>,
    builder_stages: Vec<String>,
    builder_tag: String,
    rootfs_tag: String,
}

impl TestCliBuilder {
    pub fn new() -> Self {
        Self {
            rootfs_base: test_constants::DEFAULT_ROOTFS_BASE.to_string(),
            rootfs_stages: vec![],
            builder_stages: vec![],
            builder_tag: test_constants::DEFAULT_BUILDER_TAG.to_string(),
            rootfs_tag: test_constants::DEFAULT_ROOTFS_TAG.to_string(),
        }
    }
    
    pub fn with_rootfs_base(mut self, base: &str) -> Self {
        self.rootfs_base = base.to_string();
        self
    }
    
    pub fn with_rootfs_stages(mut self, stages: &[&str]) -> Self {
        self.rootfs_stages = stages.iter().map(|s| s.to_string()).collect();
        self
    }
    
    pub fn with_builder_stages(mut self, stages: &[&str]) -> Self {
        self.builder_stages = stages.iter().map(|s| s.to_string()).collect();
        self
    }
    
    pub fn build(self) -> Cli {
        Cli {
            command: Commands::Build,
            builder_tag: self.builder_tag,
            podman_build_cache: None,
            auto_clean: false,
            pacman_cache: None,
            aur_cache: None,
            src_dir: None,
            extra_contexts: vec![],
            extra_mounts: vec![],
            rootfs_stages: self.rootfs_stages,
            rootfs_base: self.rootfs_base,
            rootfs_tag: self.rootfs_tag,
            builder_stages: self.builder_stages,
        }
    }
}

/// Creates a test command with standard environment setup
pub fn create_test_command() -> Command {
    let mut cmd = Command::cargo_bin("trls").unwrap();
    cmd.env("TRLS_SKIP_ROOT_CHECK", "1");
    cmd
}

/// Asserts that a command fails with a specific error message
pub fn assert_command_fails_with(cmd: &mut Command, expected_message: &str) {
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected_message),
        "Expected error message '{}' not found in stderr: {}",
        expected_message,
        stderr
    );
}

