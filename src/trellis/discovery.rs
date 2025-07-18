use anyhow::{anyhow, Result};
use walkdir::WalkDir;
use lru::LruCache;
use std::cell::RefCell;
use std::path::PathBuf;
use std::time::SystemTime;
use std::fs;

use crate::config::TrellisConfig;
use super::{constants::{patterns, errors}, common::TrellisMessaging};

/// Cache entry for containerfile discovery results
struct ContainerfileCacheEntry {
    path: PathBuf,
    discovery_time: SystemTime,
}

/// LRU cache for containerfile discovery with directory modification time tracking
struct ContainerfileCache {
    cache: LruCache<String, ContainerfileCacheEntry>,
    src_dir_mtime: Option<SystemTime>,
}

impl ContainerfileCache {
    fn new() -> Self {
        Self {
            cache: LruCache::new(std::num::NonZeroUsize::new(100).unwrap()),
            src_dir_mtime: None,
        }
    }

    fn is_valid(&self, src_dir: &PathBuf) -> bool {
        if let Ok(metadata) = fs::metadata(src_dir) {
            if let Ok(current_mtime) = metadata.modified() {
                return self.src_dir_mtime.map_or(false, |cached_mtime| cached_mtime >= current_mtime);
            }
        }
        false
    }

    fn update_src_dir_mtime(&mut self, src_dir: &PathBuf) {
        if let Ok(metadata) = fs::metadata(src_dir) {
            if let Ok(mtime) = metadata.modified() {
                self.src_dir_mtime = Some(mtime);
            }
        }
    }

    fn get(&mut self, group: &str) -> Option<&ContainerfileCacheEntry> {
        self.cache.get(group)
    }

    fn put(&mut self, group: String, path: PathBuf) {
        let entry = ContainerfileCacheEntry {
            path,
            discovery_time: SystemTime::now(),
        };
        self.cache.put(group, entry);
    }

    fn invalidate(&mut self) {
        self.cache.clear();
        self.src_dir_mtime = None;
    }
}

/// Handles discovery of Containerfiles in the source directory.
pub struct ContainerfileDiscovery<'a> {
    config: &'a TrellisConfig,
    cache: RefCell<ContainerfileCache>,
}

impl<'a> TrellisMessaging for ContainerfileDiscovery<'a> {}

impl<'a> ContainerfileDiscovery<'a> {
    pub fn new(config: &'a TrellisConfig) -> Self {
        Self { 
            config,
            cache: RefCell::new(ContainerfileCache::new()),
        }
    }

    /// Efficiently searches for a Containerfile with the specified group name using walkdir.
    /// 
    /// This implementation includes an LRU cache to avoid repeated directory traversals
    /// for the same group names. The cache is invalidated when the source directory
    /// modification time changes.
    /// 
    /// # Arguments
    /// 
    /// * `group` - The group name to search for (e.g., "base", "tools")
    /// 
    /// # Errors
    /// 
    /// Returns an error if the containerfile is not found or if directory traversal fails.
    pub fn find_containerfile(&self, group: &str) -> Result<String> {
        let mut cache = self.cache.borrow_mut();
        
        // Check if cache is still valid
        if !cache.is_valid(&self.config.src_dir) {
            cache.invalidate();
            cache.update_src_dir_mtime(&self.config.src_dir);
        }
        
        // Check cache first
        if let Some(entry) = cache.get(group) {
            // Verify the cached file still exists
            if entry.path.exists() {
                return Ok(entry.path.to_string_lossy().into_owned());
            } else {
                // File was deleted, remove from cache and continue with fresh search
                cache.cache.pop(group);
            }
        }
        
        // Cache miss or invalid entry - perform filesystem search
        let result = self.find_containerfile_uncached(group);
        
        // Cache the result if successful
        if let Ok(ref path_str) = result {
            let path = PathBuf::from(path_str);
            cache.put(group.to_string(), path);
        }
        
        result
    }

    /// Performs uncached containerfile discovery using walkdir.
    fn find_containerfile_uncached(&self, group: &str) -> Result<String> {
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
                "{}: {filename} (searched recursively in {} and all subdirectories). \
                 Ensure the file exists and has correct permissions. \
                 Use 'find {} -name \"{}\"' to verify file location.",
                errors::CONTAINERFILE_NOT_FOUND,
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
                "{}: {}",
            errors::MISSING_CONTAINERFILES,
                missing_files.join(", ")
            ));
        }
        
        Ok(())
    }

}
