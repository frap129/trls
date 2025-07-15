use anyhow::{anyhow, Result};
use walkdir::WalkDir;

use crate::config::TrellisConfig;
use super::{constants::patterns, common::TrellisMessaging};

/// Handles discovery of Containerfiles in the source directory.
pub struct ContainerfileDiscovery<'a> {
    config: &'a TrellisConfig,
}

impl<'a> TrellisMessaging for ContainerfileDiscovery<'a> {}

impl<'a> ContainerfileDiscovery<'a> {
    pub fn new(config: &'a TrellisConfig) -> Self {
        Self { config }
    }

    /// Efficiently searches for a Containerfile with the specified group name using walkdir.
    /// 
    /// This replaces the original recursive implementation with a more efficient approach
    /// that uses the walkdir crate for better performance and built-in cycle detection.
    /// 
    /// # Arguments
    /// 
    /// * `group` - The group name to search for (e.g., "base", "tools")
    /// 
    /// # Errors
    /// 
    /// Returns an error if the containerfile is not found or if directory traversal fails.
    pub fn find_containerfile(&self, group: &str) -> Result<String> {
        let filename = format!("{}{group}", patterns::CONTAINERFILE_PREFIX);
        
        // Use walkdir for efficient directory traversal with built-in features:
        // - Automatic cycle detection
        // - Depth limiting
        // - Error handling for inaccessible directories
        let walker = WalkDir::new(&self.config.src_dir)
            .max_depth(patterns::MAX_SEARCH_DEPTH) // Reasonable depth limit to prevent runaway searches
            .follow_links(false) // Don't follow symlinks to avoid cycles
            .into_iter()
            .filter_map(|entry| {
                match entry {
                    Ok(entry) => Some(entry),
                    Err(err) => {
                        // Log warnings for inaccessible directories but continue searching
                        self.warning(&format!("Error accessing directory: {err}"));
                        None
                    }
                }
            });

        // Search for the containerfile, collecting paths with depth for efficient sorting
        let mut found_paths_with_depth = Vec::new();
        
        for entry in walker {
            if entry.file_type().is_file() && entry.file_name() == filename.as_str() {
                let depth = entry.path().components().count();
                found_paths_with_depth.push((entry.path().to_path_buf(), depth));
            }
        }

        if found_paths_with_depth.is_empty() {
            return Err(anyhow!(
                "Containerfile not found: {filename} (searched recursively in {} and all subdirectories). \
                 Ensure the file exists and has correct permissions. \
                 Use 'find {} -name \"{}\"' to verify file location.",
                self.config.src_dir.display(),
                self.config.src_dir.display(),
                filename
            ));
        }

        // Sort by depth (descending) for most specific match
        found_paths_with_depth.sort_unstable_by_key(|(_, depth)| std::cmp::Reverse(*depth));
        
        // Return the path as string
        Ok(found_paths_with_depth[0].0.to_string_lossy().into_owned())
    }

    /// Parses a stage name that may include group syntax (group:stage).
    /// 
    /// # Arguments
    /// 
    /// * `build_stage` - Stage name, either "stage" or "group:stage"
    /// 
    /// # Returns
    /// 
    /// A tuple of (group, stage) where group equals stage if no colon is present.
    pub fn parse_stage_name(build_stage: &str) -> (String, String) {
        build_stage
            .split_once(':')
            .map(|(group, stage)| (group.to_string(), stage.to_string()))
            .unwrap_or_else(|| (build_stage.to_string(), build_stage.to_string()))
    }

    /// Validates that all required containerfiles exist for the given stages.
    /// 
    /// This performs upfront validation to fail fast if any required files are missing.
    /// 
    /// # Arguments
    /// 
    /// * `stages` - List of stage names to validate
    /// 
    /// # Errors
    /// 
    /// Returns an error if any required containerfile is not found.
    pub fn validate_stages(&self, stages: &[String]) -> Result<()> {
        let mut missing_files = Vec::new();
        
        for stage in stages {
            let (group, _) = Self::parse_stage_name(stage);
            if self.find_containerfile(&group).is_err() {
                missing_files.push(format!("{}{group}", patterns::CONTAINERFILE_PREFIX));
            }
        }
        
        if !missing_files.is_empty() {
            return Err(anyhow!(
                "Missing required containerfiles: {}",
                missing_files.join(", ")
            ));
        }
        
        Ok(())
    }

}
