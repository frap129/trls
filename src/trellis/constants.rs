//! Constants and default values used throughout trellis.

/// Default paths and configuration values
pub mod paths {
    /// Default location for the trellis configuration file
    pub const DEFAULT_CONFIG_PATH: &str = "/etc/trellis/trellis.toml";
    
    /// Default location for trellis hooks directory
    pub const DEFAULT_HOOKS_DIR: &str = "/etc/trellis/hooks.d";
    
    /// Default source directory for containerfiles
    pub const DEFAULT_SRC_DIR: &str = "/var/lib/trellis/src";
    
    /// Default pacman cache directory
    pub const DEFAULT_PACMAN_CACHE: &str = "/var/cache/pacman/pkg";
    
    /// Default AUR cache directory
    pub const DEFAULT_AUR_CACHE: &str = "/var/cache/trellis/aur";
}

/// Container and image related constants
pub mod containers {
    /// Default builder container tag
    pub const DEFAULT_BUILDER_TAG: &str = "trellis-builder";
    
    /// Default rootfs container tag
    pub const DEFAULT_ROOTFS_TAG: &str = "trellis-rootfs";
    
    /// Container registry prefix for local images
    pub const LOCALHOST_PREFIX: &str = "localhost/";
    
    /// Builder image tag prefix for intermediate images
    pub const BUILDER_PREFIX: &str = "trellis-builder";
    
    /// Stage image tag prefix for intermediate images
    pub const STAGE_PREFIX: &str = "trellis-stage";
}

/// Environment variable names
pub mod env_vars {
    /// Environment variable to skip root check (for testing)
    pub const SKIP_ROOT_CHECK: &str = "TRLS_SKIP_ROOT_CHECK";
    
    /// Environment variable to override config file path
    pub const CONFIG_PATH: &str = "TRELLIS_CONFIG";
    
    /// Buildah layers environment variable for cache control
    pub const BUILDAH_LAYERS: &str = "BUILDAH_LAYERS";
}

/// File and path patterns
pub mod patterns {
    /// Containerfile filename pattern
    pub const CONTAINERFILE_PREFIX: &str = "Containerfile.";
    
    /// Maximum directory traversal depth for safety
    pub const MAX_SEARCH_DEPTH: usize = 10;
}