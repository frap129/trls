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

/// File and path patterns
pub mod patterns {
    /// Containerfile filename pattern
    pub const CONTAINERFILE_PREFIX: &str = "Containerfile.";

    /// Maximum directory traversal depth for safety
    pub const MAX_SEARCH_DEPTH: usize = 20;
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
}
