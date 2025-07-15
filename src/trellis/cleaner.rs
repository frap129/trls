use std::process::Command;
use anyhow::{anyhow, Context, Result};

use crate::config::TrellisConfig;
use super::{
    common::TrellisMessaging,
    constants::containers,
};

/// Mode for cleaning container images.
#[derive(Debug, Clone, Copy)]
pub enum CleanMode {
    /// Remove all trls-generated images
    Full,
    /// Remove only intermediate images, keep final builder/rootfs tags
    Auto,
}

/// Handles cleanup of container images.
pub struct ImageCleaner<'a> {
    config: &'a TrellisConfig,
}

impl<'a> TrellisMessaging for ImageCleaner<'a> {}

impl<'a> ImageCleaner<'a> {
    pub fn new(config: &'a TrellisConfig) -> Self {
        Self { config }
    }

    /// Removes all trellis-generated container images.
    pub fn clean_all(&self) -> Result<()> {
        self.msg("Cleaning trls-generated images...");
        
        let removed_count = self.clean_images(CleanMode::Full)?;
        
        if removed_count == 0 {
            self.msg("No trls-generated images found to clean");
        } else {
            self.msg(&format!("Cleanup completed - removed {} images", removed_count));
        }
        
        Ok(())
    }

    /// Automatically cleans intermediate images if auto-cleanup is enabled.
    pub fn auto_clean(&self) -> Result<()> {
        if !self.config.auto_clean {
            return Ok(());
        }
        
        let removed_count = self.clean_images(CleanMode::Auto)?;
        
        if removed_count > 0 {
            self.msg(&format!("Auto-cleanup removed {} intermediate images", removed_count));
        }
        
        Ok(())
    }

    /// Core image cleaning logic with optimized filtering and batch operations.
    fn clean_images(&self, mode: CleanMode) -> Result<u32> {
        let mode_desc = match mode {
            CleanMode::Full => "all trls-generated",
            CleanMode::Auto => "intermediate trls-generated",
        };
        
        let output = Command::new("podman")
            .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
            .output()
            .context("Failed to list podman images")?;
        
        if !output.status.success() {
            return Err(anyhow!("Failed to list images: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }
        
        // Pre-compute expected image names once for efficiency
        let expected_builder = format!("{}{}:latest", containers::LOCALHOST_PREFIX, self.config.builder_tag);
        let expected_rootfs = format!("{}{}:latest", containers::LOCALHOST_PREFIX, self.config.rootfs_tag);
        
        let image_list = String::from_utf8_lossy(&output.stdout);
        
        // Optimized filtering: process images and collect those to remove in one pass
        let images_to_remove: Vec<&str> = image_list
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if !line.is_empty() && self.should_remove_image(line, mode, &expected_builder, &expected_rootfs) {
                    Some(line)
                } else {
                    None
                }
            })
            .collect();
        
        if images_to_remove.is_empty() {
            return Ok(0);
        }
        
        self.msg(&format!("Found {} {} images to remove", images_to_remove.len(), mode_desc));
        
        // Use optimized batch removal
        self.remove_images_batch(&images_to_remove)
    }

    /// Determines whether an image should be removed based on the cleanup mode.
    fn should_remove_image(&self, image: &str, mode: CleanMode, expected_builder: &str, expected_rootfs: &str) -> bool {
        // Check if this is a trellis-generated image
        let builder_prefix = format!("{}{}", containers::LOCALHOST_PREFIX, containers::BUILDER_PREFIX);
        let stage_prefix = format!("{}{}", containers::LOCALHOST_PREFIX, containers::STAGE_PREFIX);
        
        let is_trellis = image.starts_with(&builder_prefix) 
            || image.starts_with(&stage_prefix)
            || image == expected_builder 
            || image == expected_rootfs;
        
        if !is_trellis {
            return false;
        }
        
        match mode {
            CleanMode::Full => true, // Remove all trellis images
            CleanMode::Auto => {
                // Only remove intermediate images, preserve final tags
                image != expected_builder && image != expected_rootfs
            }
        }
    }

    /// Optimized batch image removal with fallback to individual removal.
    fn remove_images_batch(&self, images: &[&str]) -> Result<u32> {
        if images.len() == 1 {
            return self.remove_single_image(images[0]);
        }

        // Try to remove all images in a single command first
        let mut cmd = Command::new("podman");
        cmd.args(["rmi", "-f"]);
        cmd.args(images);
        
        match cmd.output() {
            Ok(output) if output.status.success() => {
                self.msg(&format!("Batch removed {} images", images.len()));
                Ok(images.len() as u32)
            }
            Ok(output) => {
                // Batch removal failed, try individual removal
                let stderr = String::from_utf8_lossy(&output.stderr);
                self.msg(&format!("Batch removal failed: {stderr}"));
                self.msg("Trying individual removal...");
                
                let mut removed_count = 0;
                for image in images {
                    removed_count += self.remove_single_image(image)?;
                }
                Ok(removed_count)
            }
            Err(e) => {
                // Command execution failed
                self.warning(&format!("Failed to execute batch removal: {e}"));
                self.msg("Trying individual removal...");
                
                let mut removed_count = 0;
                for image in images {
                    removed_count += self.remove_single_image(image)?;
                }
                Ok(removed_count)
            }
        }
    }

    /// Removes a single image with detailed error reporting.
    fn remove_single_image(&self, image: &str) -> Result<u32> {
        let output = Command::new("podman")
            .args(["rmi", "-f", image])
            .output()
            .context("Failed to remove image")?;
            
        if output.status.success() {
            self.msg(&format!("Removed image: {}", image));
            Ok(1)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.warning(&format!("Failed to remove image {}: {}", image, stderr));
            Ok(0)
        }
    }

}
