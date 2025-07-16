use anyhow::{Context, Result};
use clap::Parser;
use std::{io::{self}, process};

mod cli;
mod config;
mod trellis;

use cli::Cli;
use trellis::{TrellisApp, constants::env_vars, common::{TrellisMessager, TrellisMessaging}};

/// Checks if the current process is running as root with multiple fallback methods.
/// 
/// This function tries multiple approaches to determine root privileges:
/// 1. Filesystem metadata check via /proc/self (Linux)
/// 2. Direct libc getuid() call as fallback
/// 3. Environment variable check for testing
/// 
/// # Errors
/// 
/// Returns an error only if all detection methods fail.
fn is_running_as_root() -> Result<bool> {
    // Skip root check in test mode
    if std::env::var(env_vars::SKIP_ROOT_CHECK).is_ok() {
        return Ok(true);
    }
    
    // Method 1: Try filesystem metadata (Linux-specific)
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        
        if let Ok(metadata) = std::fs::metadata("/proc/self") {
            return Ok(metadata.uid() == 0);
        }
    }
    
    // Method 2: Fallback to direct libc call
    #[cfg(unix)]
    {
        unsafe {
            return Ok(libc::getuid() == 0);
        }
    }
}

/// Prompts the user to continue when not running as root.
/// 
/// Displays a warning about potential permission issues and asks for confirmation.
/// Uses buffered reading for better performance and reliability.
/// 
/// # Errors
/// 
/// Returns an error if stdin/stderr operations fail.
fn prompt_continue_as_non_root() -> Result<bool> {
    use std::io::BufRead;
    
    let messager = TrellisMessager::new();
    messager.warning("Running trls as non-root user");
    messager.warning("Container operations may fail or require additional permissions");
    messager.prompt("Do you want to continue? [y/N]: ");
    
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut input = String::new();
    handle.read_line(&mut input)
        .context("Failed to read user input")?;
    
    let response = input.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let messager = TrellisMessager::new();
    
    // Check if running as root and prompt user if not
    match is_running_as_root() {
        Ok(true) => {}, // Running as root, continue normally
        Ok(false) => {
            if !prompt_continue_as_non_root()? {
                messager.error("Aborted by user");
                process::exit(1);
            }
        }
        Err(e) => {
            messager.warning(&format!("Could not determine if running as root: {e}"));
            messager.warning("Continuing anyway, but container operations may fail");
        }
    }
    
    let app = TrellisApp::new(cli)?;
    
    let result = app.run();
    
    match result {
        Ok(_) => {
            messager.msg("Successful");
            Ok(())
        }
        Err(e) => {
            messager.error(&format!("{e}"));
            process::exit(1);
        }
    }
}
