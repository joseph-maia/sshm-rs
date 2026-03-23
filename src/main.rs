mod cli;
mod config;
mod connectivity;
mod credentials;
mod favorites;
mod groups;
mod history;
mod snippets;
mod term;
mod theme;
mod ui;
mod update;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli::run(cli)
}
