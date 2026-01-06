use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Increase verbosity. Can be used multiple times (e.g., -v, -vv, -vvv).
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Serve
    Serve,
    /// Index the whole file system
    Index {
        root_path: PathBuf
    }
}
