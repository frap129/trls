pub mod cli;
pub mod config;
pub mod trellis;

// Re-export main types for easier access
pub use trellis::{TrellisApp, Trellis, ContainerBuilder, BuildType, ImageCleaner, ContainerRunner};
pub use trellis::discovery::ContainerfileDiscovery;