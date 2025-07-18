use anyhow::{anyhow, Result};
use lru::LruCache;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use walkdir::WalkDir;

use super::{
    common::TrellisMessaging,
    constants::{errors, patterns},
};
use crate::config::TrellisConfig;

/// Cache entry for containerfile discovery results
struct ContainerfileCacheEntry {
    path: PathBuf,
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
                return self
                    .src_dir_mtime
                    .is_some_and(|cached_mtime| cached_mtime >= current_mtime);
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
        let entry = ContainerfileCacheEntry { path };
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
    pub fn find_containerfile(&self, group: &str) -> Result<PathBuf> {
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
                return Ok(entry.path.clone());
            } else {
                // File was deleted, remove from cache and continue with fresh search
                cache.cache.pop(group);
            }
        }

        // Cache miss or invalid entry - perform filesystem search
        let result = self.find_containerfile_uncached(group);

        // Cache the result if successful
        if let Ok(ref path) = result {
            cache.put(group.to_string(), path.clone());
        }

        result
    }

    /// Performs uncached containerfile discovery using walkdir.
    fn find_containerfile_uncached(&self, group: &str) -> Result<PathBuf> {
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

        // Return the path as PathBuf
        Ok(found_paths_with_depth[0].0.clone())
    }

    /// Efficiently discovers multiple containerfiles with early termination.
    ///
    /// This method is optimized for batch discovery operations like validate_stages()
    /// where we know all required groups upfront. It terminates directory traversal
    /// early once all required files are found.
    ///
    /// # Arguments
    ///
    /// * `groups` - Set of group names to search for
    ///
    /// # Returns
    ///
    /// A HashMap mapping group names to their discovered paths
    pub fn find_multiple_containerfiles(
        &self,
        groups: &[String],
    ) -> Result<HashMap<String, PathBuf>> {
        if groups.is_empty() {
            return Ok(HashMap::new());
        }

        let mut found = HashMap::new();
        let mut remaining: HashSet<String> = groups.iter().cloned().collect();
        let mut walker_count = 0;

        // Use walkdir for efficient directory traversal
        let walker = WalkDir::new(&self.config.src_dir)
            .max_depth(patterns::MAX_SEARCH_DEPTH)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| match entry {
                Ok(entry) => Some(entry),
                Err(err) => {
                    self.warning(&format!("Error accessing directory: {err}"));
                    None
                }
            });

        for entry in walker {
            walker_count += 1;

            // Progress reporting for large directory trees
            if walker_count % 1000 == 0 {
                self.msg(&format!("Searched {walker_count} directories..."));
            }

            if entry.file_type().is_file() {
                if let Some(filename) = entry.file_name().to_str() {
                    if let Some(group) = self.extract_group_from_filename(filename) {
                        if remaining.contains(&group) {
                            let depth = entry.path().components().count();

                            // If we already found this group, keep the deeper (more specific) one
                            match found.get(&group) {
                                Some((_, existing_depth)) if depth <= *existing_depth => continue,
                                _ => {}
                            }

                            found.insert(group.clone(), (entry.path().to_path_buf(), depth));
                            remaining.remove(&group);

                            // Early termination when all files are found
                            if remaining.is_empty() {
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Convert to final result format (remove depth information)
        let result: HashMap<String, PathBuf> = found
            .into_iter()
            .map(|(group, (path, _))| (group, path))
            .collect();

        Ok(result)
    }

    /// Extracts group name from a containerfile filename.
    ///
    /// # Arguments
    ///
    /// * `filename` - The filename to parse (e.g., "Containerfile.base")
    ///
    /// # Returns
    ///
    /// The group name if the filename matches the containerfile pattern
    fn extract_group_from_filename(&self, filename: &str) -> Option<String> {
        if let Some(group) = filename.strip_prefix(patterns::CONTAINERFILE_PREFIX) {
            if !group.is_empty() {
                return Some(group.to_string());
            }
        }
        None
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
    /// Uses batch discovery with early termination for improved performance.
    ///
    /// # Arguments
    ///
    /// * `stages` - List of stage names to validate
    ///
    /// # Errors
    ///
    /// Returns an error if any required containerfile is not found.
    pub fn validate_stages(&self, stages: &[String]) -> Result<()> {
        if stages.is_empty() {
            return Ok(());
        }

        // Extract unique group names from stages
        let groups: Vec<String> = stages
            .iter()
            .map(|stage| Self::parse_stage_name(stage).0)
            .collect::<HashSet<_>>() // Remove duplicates
            .into_iter()
            .collect();

        // Use batch discovery for efficiency
        let found_files = self.find_multiple_containerfiles(&groups)?;

        // Check for missing files
        let mut missing_files = Vec::new();
        for group in &groups {
            if !found_files.contains_key(group) {
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
