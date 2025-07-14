use anyhow::Result;
use clap::Parser;
use std::process;

mod cli;
mod config;
mod trellis;

use cli::Cli;
use trellis::TrellisApp;

fn main() -> Result<()> {
    let cli = Cli::parse();
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