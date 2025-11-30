pub mod cli;
pub mod config;
pub mod trellis;

// Re-export main types for easier access
pub use trellis::discovery::ContainerfileDiscovery;
pub use trellis::{
    resolve_image_tag, ContainerBuilder, ContainerRunner, ImageCleaner, Trellis, TrellisApp,
};
