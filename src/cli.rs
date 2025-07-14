use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "trellis")]
#[command(about = "A container build system for multi-stage builds")]
#[command(long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Name of the tag to use for the pacstrap container
    #[arg(long, default_value = "trellis-builder")]
    pub builder_tag: String,

    /// Enable/Disable podman build cache
    #[arg(long)]
    pub podman_build_cache: Option<bool>,

    /// Path to a persistent pacman package cache
    #[arg(long)]
    pub pacman_cache: Option<PathBuf>,

    /// Path to use as a persistent AUR package build cache
    #[arg(long)]
    pub aur_cache: Option<PathBuf>,

    /// Path to the directory with Containerfiles and setup files
    #[arg(long)]
    pub src_dir: Option<PathBuf>,

    /// A comma delimited list of container build contexts
    #[arg(long, value_delimiter = ',')]
    pub extra_contexts: Vec<String>,

    /// A comma delimited list of directories or files to be bind mounted
    #[arg(long, value_delimiter = ',')]
    pub extra_mounts: Vec<PathBuf>,

    /// A comma delimited list of the image stages to build
    #[arg(long, value_delimiter = ',')]
    pub rootfs_stages: Vec<String>,

    /// Name of the tag to use for the rootfs container
    #[arg(long, default_value = "trellis-rootfs")]
    pub rootfs_tag: String,

    /// A comma delimited list of the builder image stages to build
    #[arg(long, value_delimiter = ',')]
    pub builder_stages: Vec<String>,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// (Re-)Build pacstrap container that's used by the other commands
    BuildBuilder,
    /// Build all requested stages from files in --src-dir
    Build,
    /// Remove unused container images
    Clean,
    /// Run cmd in the latest --rootfs-tag container
    Run { args: Vec<String> },
    /// A macro command that runs build and bootc upgrade
    Update,
}