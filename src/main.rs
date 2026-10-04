mod cli;
mod index;
mod mcp;
mod memory;
mod vault;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    cli::run(args)
}
