use super::lib::TrellisConfig;
use crate::trellis::constants::errors;
use anyhow::{anyhow, Result};

/// Centralized configuration validator.
///
/// This module consolidates all configuration validation logic in one place,
/// providing comprehensive validation with clear error messages.
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validates a complete TrellisConfig for all potential issues.
    ///
    /// This performs comprehensive validation including:
    /// - Path validation
    /// - Cross-dependency validation
    ///
    /// Note: Stage validation is not performed here as empty stages are allowed
    /// during config creation and should only be validated when operations require them.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration to validate
    ///
    /// # Errors
    ///
    /// Returns an error if any validation check fails
    pub fn validate_complete(config: &TrellisConfig) -> Result<()> {
        Self::validate_paths(config)?;
        Self::validate_cross_dependencies(config)?;
        Ok(())
    }

    /// Validates that stage lists are not empty when they should contain stages.
    ///
    /// # Arguments
    ///
    /// * `stages` - List of stage names to validate
    /// * `stage_type` - Type of stages for error messaging ("builder" or "rootfs")
    ///
    /// # Errors
    ///
    /// Returns an error if the stage list is empty
    pub fn validate_stages(stages: &[String], stage_type: &str) -> Result<()> {
        if stages.is_empty() {
            let error_msg = match stage_type {
                "builder" => errors::NO_BUILDER_STAGES,
                "rootfs" => errors::NO_ROOTFS_STAGES,
                _ => "No stages defined",
            };
            return Err(anyhow!(error_msg));
        }
        Ok(())
    }

    /// Validates path-related configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration to validate
    ///
    /// # Errors
    ///
    /// Returns an error if any path validation fails
    fn validate_paths(config: &TrellisConfig) -> Result<()> {
        // Validate source directory exists
        if !config.src_dir.exists() {
            return Err(anyhow!(
                "Source directory does not exist: {}",
                config.src_dir.display()
            ));
        }

        // Validate source directory is actually a directory
        if !config.src_dir.is_dir() {
            return Err(anyhow!(
                "Source path is not a directory: {}",
                config.src_dir.display()
            ));
        }

        // Validate cache directories if specified
        if let Some(ref pacman_cache) = config.pacman_cache {
            if let Some(parent) = pacman_cache.parent() {
                if !parent.exists() {
                    return Err(anyhow!(
                        "Pacman cache parent directory does not exist: {}",
                        parent.display()
                    ));
                }
            }
        }

        if let Some(ref aur_cache) = config.aur_cache {
            if let Some(parent) = aur_cache.parent() {
                if !parent.exists() {
                    return Err(anyhow!(
                        "AUR cache parent directory does not exist: {}",
                        parent.display()
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validates cross-dependencies between configuration options.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration to validate
    ///
    /// # Errors
    ///
    /// Returns an error if any cross-dependency validation fails
    fn validate_cross_dependencies(config: &TrellisConfig) -> Result<()> {
        // Validate tag names are not empty
        if config.builder_tag.is_empty() {
            return Err(anyhow!("Builder tag cannot be empty"));
        }

        if config.rootfs_tag.is_empty() {
            return Err(anyhow!("Rootfs tag cannot be empty"));
        }

        // Validate that builder and rootfs tags are different
        if config.builder_tag == config.rootfs_tag {
            return Err(anyhow!(
                "Builder and rootfs tags must be different: {}",
                config.builder_tag
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_config() -> (TrellisConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = TrellisConfig {
            builder_stages: vec!["base".to_string()],
            rootfs_stages: vec!["stage1".to_string()],
            rootfs_base: "scratch".to_string(),
            extra_contexts: vec![],
            extra_mounts: vec![],
            builder_tag: "test-builder".to_string(),
            rootfs_tag: "test-rootfs".to_string(),
            podman_build_cache: true,
            auto_clean: false,
            pacman_cache: None,
            aur_cache: None,
            src_dir: temp_dir.path().to_path_buf(),
            hooks_dir: None,
            quiet: false,
        };
        (config, temp_dir)
    }

    #[test]
    fn test_validate_complete_success() {
        let (config, _temp_dir) = create_test_config();
        let result = ConfigValidator::validate_complete(&config);
        if let Err(ref e) = result {
            println!("Validation error: {e}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_stages_empty_builder() {
        let result = ConfigValidator::validate_stages(&[], "builder");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("builder"));
    }

    #[test]
    fn test_validate_stages_empty_rootfs() {
        let result = ConfigValidator::validate_stages(&[], "rootfs");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rootfs"));
    }

    #[test]
    fn test_validate_stages_non_empty() {
        let stages = vec!["stage1".to_string(), "stage2".to_string()];
        assert!(ConfigValidator::validate_stages(&stages, "builder").is_ok());
    }

    #[test]
    fn test_validate_paths_nonexistent_src_dir() {
        let (mut config, _temp_dir) = create_test_config();
        config.src_dir = PathBuf::from("/nonexistent/path");

        let result = ConfigValidator::validate_paths(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_validate_cross_dependencies_same_tags() {
        let (mut config, _temp_dir) = create_test_config();
        config.builder_tag = "same-tag".to_string();
        config.rootfs_tag = "same-tag".to_string();

        let result = ConfigValidator::validate_cross_dependencies(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be different"));
    }

    #[test]
    fn test_validate_cross_dependencies_empty_tag() {
        let (mut config, _temp_dir) = create_test_config();
        config.builder_tag = "".to_string();

        let result = ConfigValidator::validate_cross_dependencies(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_cross_dependencies_empty_rootfs_tag() {
        let (mut config, _temp_dir) = create_test_config();
        config.rootfs_tag = "".to_string();

        let result = ConfigValidator::validate_cross_dependencies(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_stages_unknown_type() {
        let stages = vec!["stage1".to_string()];
        let result = ConfigValidator::validate_stages(&stages, "unknown");
        assert!(result.is_ok()); // Should still pass for unknown stage types
    }

    #[test]
    fn test_validate_stages_empty_unknown_type() {
        let result = ConfigValidator::validate_stages(&[], "unknown");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No stages defined"));
    }

    #[test]
    fn test_validate_paths_src_dir_is_file() {
        use std::fs::File;
        
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_directory");
        File::create(&file_path).unwrap();
        
        let (mut config, _temp_dir) = create_test_config();
        config.src_dir = file_path;

        let result = ConfigValidator::validate_paths(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_validate_paths_pacman_cache_parent_not_exists() {
        let (mut config, _temp_dir) = create_test_config();
        config.pacman_cache = Some(PathBuf::from("/nonexistent/parent/cache"));

        let result = ConfigValidator::validate_paths(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Pacman cache parent directory does not exist"));
    }

    #[test]
    fn test_validate_paths_aur_cache_parent_not_exists() {
        let (mut config, _temp_dir) = create_test_config();
        config.aur_cache = Some(PathBuf::from("/nonexistent/parent/cache"));

        let result = ConfigValidator::validate_paths(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("AUR cache parent directory does not exist"));
    }

    #[test]
    fn test_validate_paths_cache_dirs_with_valid_parents() {
        let temp_dir = TempDir::new().unwrap();
        let (mut config, _temp_dir) = create_test_config();
        
        // Set cache directories with existing parent
        config.pacman_cache = Some(temp_dir.path().join("pacman_cache"));
        config.aur_cache = Some(temp_dir.path().join("aur_cache"));

        let result = ConfigValidator::validate_paths(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_paths_cache_dirs_none() {
        let (mut config, _temp_dir) = create_test_config();
        config.pacman_cache = None;
        config.aur_cache = None;

        let result = ConfigValidator::validate_paths(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_complete_with_path_error() {
        let (mut config, _temp_dir) = create_test_config();
        config.src_dir = PathBuf::from("/nonexistent/path");

        let result = ConfigValidator::validate_complete(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_complete_with_cross_dependency_error() {
        let (mut config, _temp_dir) = create_test_config();
        config.builder_tag = "same".to_string();
        config.rootfs_tag = "same".to_string();

        let result = ConfigValidator::validate_complete(&config);
        assert!(result.is_err());
    }
}
