use anyhow::Result;
use clap::Parser;
use std::{io::{self, Write}, process};

mod cli;
mod config;
mod trellis;

use cli::Cli;
use trellis::TrellisApp;

fn is_running_as_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

fn prompt_continue_as_non_root() -> Result<bool> {
    eprintln!("====> WARNING: Running trls as non-root user");
    eprintln!("====> Container operations may fail or require additional permissions");
    eprint!("====> Do you want to continue? [y/N]: ");
    io::stderr().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let response = input.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Check if running as root and prompt user if not (skip in test mode)
    if !is_running_as_root() && std::env::var("TRLS_SKIP_ROOT_CHECK").is_err() {
        if !prompt_continue_as_non_root()? {
            eprintln!("====> Aborted by user");
            process::exit(1);
        }
    }
    
    let app = TrellisApp::new(cli)?;
    
    let result = app.run();
    
    match result {
        Ok(_) => {
            println!("====> Successful");
            Ok(())
        }
        Err(e) => {
            eprintln!("====> ERROR: {e}");
            process::exit(1);
        }
    }
}