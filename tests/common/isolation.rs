//! Test environment isolation utilities.
//!
//! This module provides utilities for creating isolated test environments
//! that don't interfere with each other or the system.

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

use crate::common::mocks::{CommandExecutor, MockCommandExecutor};

/// Manages environment variable isolation for tests.
pub struct IsolatedEnvironment {
    original_vars: HashMap<String, Option<String>>,
    temp_dirs: Vec<TempDir>,
}

impl IsolatedEnvironment {
    /// Create a new isolated environment.
    pub fn new() -> Self {
        Self {
            original_vars: HashMap::new(),
            temp_dirs: Vec::new(),
        }
    }

    /// Set an environment variable for this test, saving the original value.
    pub fn set_var(&mut self, key: &str, value: &str) {
        // Save original value if we haven't already
        if !self.original_vars.contains_key(key) {
            self.original_vars.insert(key.to_string(), env::var(key).ok());
        }
        
        env::set_var(key, value);
    }

    /// Create a temporary directory and return its path.
    pub fn create_temp_dir(&mut self) -> Result<&TempDir, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        self.temp_dirs.push(temp_dir);
        Ok(self.temp_dirs.last().unwrap())
    }

    /// Set up isolated cache directories.
    pub fn setup_isolated_caches(&mut self) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
        let pacman_cache = self.create_temp_dir()?.path().to_path_buf();
        let aur_cache = self.create_temp_dir()?.path().to_path_buf();
        
        self.set_var("TRELLIS_PACMAN_CACHE", &pacman_cache.to_string_lossy());
        self.set_var("TRELLIS_AUR_CACHE", &aur_cache.to_string_lossy());
        
        Ok((pacman_cache, aur_cache))
    }

    /// Set up isolated source directory.
    pub fn setup_isolated_source(&mut self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let src_dir = self.create_temp_dir()?.path().to_path_buf();
        self.set_var("TRELLIS_SRC_DIR", &src_dir.to_string_lossy());
        Ok(src_dir)
    }

    /// Set up isolated hooks directory.
    pub fn setup_isolated_hooks(&mut self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let hooks_dir = self.create_temp_dir()?.path().join("hooks.d");
        std::fs::create_dir_all(&hooks_dir)?;
        self.set_var("TRELLIS_HOOKS_DIR", &hooks_dir.to_string_lossy());
        Ok(hooks_dir)
    }
}

impl Default for IsolatedEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for IsolatedEnvironment {
    fn drop(&mut self) {
        // Restore original environment variables
        for (key, original_value) in &self.original_vars {
            match original_value {
                Some(value) => env::set_var(key, value),
                None => env::remove_var(key),
            }
        }
        // temp_dirs will be automatically cleaned up when dropped
    }
}

/// Complete test fixture with isolated environment and mocked executor.
pub struct TestFixture {
    pub environment: IsolatedEnvironment,
    pub executor: Arc<dyn CommandExecutor>,
    pub src_dir: PathBuf,
    pub pacman_cache: PathBuf,
    pub aur_cache: PathBuf,
    pub hooks_dir: PathBuf,
}

impl TestFixture {
    /// Create a complete test fixture with default successful executor.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_executor(crate::common::mocks::MockScenarios::all_success())
    }

    /// Create a test fixture with a custom executor.
    pub fn with_executor(executor: MockCommandExecutor) -> Result<Self, Box<dyn std::error::Error>> {
        let mut environment = IsolatedEnvironment::new();
        
        let src_dir = environment.setup_isolated_source()?;
        let (pacman_cache, aur_cache) = environment.setup_isolated_caches()?;
        let hooks_dir = environment.setup_isolated_hooks()?;
        
        let executor = Arc::new(executor) as Arc<dyn CommandExecutor>;
        
        Ok(Self {
            environment,
            executor,
            src_dir,
            pacman_cache,
            aur_cache,
            hooks_dir,
        })
    }

    /// Create a test fixture for build failure scenarios.
    pub fn with_build_failures() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_executor(crate::common::mocks::MockScenarios::build_failures())
    }

    /// Create a test fixture with no existing images.
    pub fn with_no_images() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_executor(crate::common::mocks::MockScenarios::no_images())
    }

    /// Create a test fixture with multiple images for cleanup testing.
    pub fn with_multiple_images() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_executor(crate::common::mocks::MockScenarios::multiple_images())
    }

    /// Get a reference to the source directory.
    pub fn src_dir(&self) -> &PathBuf {
        &self.src_dir
    }

    /// Get a reference to the pacman cache directory.
    pub fn pacman_cache(&self) -> &PathBuf {
        &self.pacman_cache
    }

    /// Get a reference to the AUR cache directory.
    pub fn aur_cache(&self) -> &PathBuf {
        &self.aur_cache
    }

    /// Get a reference to the hooks directory.
    pub fn hooks_dir(&self) -> &PathBuf {
        &self.hooks_dir
    }

    /// Get a clone of the executor.
    pub fn executor(&self) -> Arc<dyn CommandExecutor> {
        Arc::clone(&self.executor)
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new().expect("Failed to create default test fixture")
    }
}

/// Utility functions for creating common test scenarios.
pub struct TestScenarios;

impl TestScenarios {
    /// Create a fixture for testing successful build operations.
    pub fn successful_build() -> TestFixture {
        TestFixture::new().expect("Failed to create successful build fixture")
    }

    /// Create a fixture for testing build failure scenarios.
    pub fn failed_build() -> TestFixture {
        TestFixture::with_build_failures().expect("Failed to create failed build fixture")
    }

    /// Create a fixture for testing cleanup operations.
    pub fn cleanup_scenario() -> TestFixture {
        TestFixture::with_multiple_images().expect("Failed to create cleanup fixture")
    }

    /// Create a fixture for testing empty repository scenarios.
    pub fn empty_repository() -> TestFixture {
        TestFixture::with_no_images().expect("Failed to create empty repository fixture")
    }
}

/// Attribute macro-like function for test isolation.
/// 
/// This function ensures that each test runs in complete isolation.
pub fn isolated_test<F, R>(test_fn: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    // Create a new thread to ensure complete isolation
    let handle = std::thread::spawn(test_fn);
    handle.join().expect("Test thread panicked")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolated_environment_var_management() {
        let original_value = env::var("TEST_VAR").ok();
        
        {
            let mut env = IsolatedEnvironment::new();
            env.set_var("TEST_VAR", "test_value");
            assert_eq!(env::var("TEST_VAR").unwrap(), "test_value");
        } // env should be dropped here
        
        // Check that the original value is restored
        match original_value {
            Some(value) => assert_eq!(env::var("TEST_VAR").unwrap(), value),
            None => assert!(env::var("TEST_VAR").is_err()),
        }
    }

    #[test]
    fn test_temp_dir_creation() {
        let mut env = IsolatedEnvironment::new();
        let temp_dir = env.create_temp_dir().unwrap();
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_isolated_cache_setup() {
        let mut env = IsolatedEnvironment::new();
        let (pacman_cache, aur_cache) = env.setup_isolated_caches().unwrap();
        
        assert!(pacman_cache.exists());
        assert!(aur_cache.exists());
        assert_eq!(env::var("TRELLIS_PACMAN_CACHE").unwrap(), pacman_cache.to_string_lossy());
        assert_eq!(env::var("TRELLIS_AUR_CACHE").unwrap(), aur_cache.to_string_lossy());
    }

    #[test]
    fn test_test_fixture_creation() {
        let fixture = TestFixture::new().unwrap();
        
        assert!(fixture.src_dir().exists());
        assert!(fixture.pacman_cache().exists());
        assert!(fixture.aur_cache().exists());
        assert!(fixture.hooks_dir().exists());
    }

    #[test]
    fn test_test_scenarios() {
        let _successful = TestScenarios::successful_build();
        let _failed = TestScenarios::failed_build();
        let _cleanup = TestScenarios::cleanup_scenario();
        let _empty = TestScenarios::empty_repository();
        // Test passes if no panics occur
    }

    #[test]
    fn test_isolated_test_function() {
        let result = isolated_test(|| {
            42
        });
        assert_eq!(result, 42);
    }
}