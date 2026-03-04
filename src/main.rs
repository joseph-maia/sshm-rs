mod cli;
mod config;
mod connectivity;
mod history;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::run(cli)
}
