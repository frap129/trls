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

/// Podman command constants
pub mod commands {
    /// Main podman command
    pub const PODMAN_CMD: &str = "podman";
    
    /// Build subcommand
    pub const BUILD_SUBCMD: &str = "build";
    
    /// Remove image subcommand
    pub const RMI_SUBCMD: &str = "rmi";
    
    /// Run container subcommand
    pub const RUN_SUBCMD: &str = "run";
    
    /// Images subcommand
    pub const IMAGES_SUBCMD: &str = "images";
    
    /// Version subcommand
    pub const VERSION_SUBCMD: &str = "--version";
}

/// Common error messages
pub mod errors {
    /// No rootfs stages defined error
    pub const NO_ROOTFS_STAGES: &str = "No rootfs stages defined";
    
    /// No builder stages defined error
    pub const NO_BUILDER_STAGES: &str = "No builder stages defined";
    
    /// Missing required containerfiles error
    pub const MISSING_CONTAINERFILES: &str = "Missing required containerfiles";
    
    /// Containerfile not found error
    pub const CONTAINERFILE_NOT_FOUND: &str = "Containerfile not found";
    
    /// Podman not available error
    pub const PODMAN_NOT_AVAILABLE: &str = "Podman is not available or not working correctly";
}