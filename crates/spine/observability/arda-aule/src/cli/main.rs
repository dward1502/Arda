#![warn(rust_2018_idioms)]
#![recursion_limit = "256"]

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "arda-aule")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Run {
        #[arg(long)]
        task_type: String,
        #[arg(long)]
        description: String,
    },
    Status,
}

pub fn run() -> anyhow::Result<()> {
    Ok(())
}

pub(crate) mod commands;
