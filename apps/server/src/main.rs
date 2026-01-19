mod cli;
mod command;
mod config;
mod constants;
mod error;
mod indexer;
mod session;

use error::WrapErr;

use clap::CommandFactory;
use clap::Parser;

// NOTE, if built using MUSL, it's probably necessary to change to use a different
// allocator just like what ripgrep does: https://github.com/BurntSushi/ripgrep/blob/0a88cccd5188074de96f54a4b6b44a63971ac157/crates/core/main.rs#L40

#[tokio::main]
async fn main() -> error::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let cfg = config::Config::load().context("Load configuration error")?;
    let command_line = cli::Cli::parse();

    if let Some(command) = command_line.command {
        let cmd: Box<dyn command::Command> = match command {
            cli::Commands::Serve => Box::new(command::ServeCommand::new(cfg)),
            cli::Commands::Index { root_path } => {
                Box::new(command::IndexCommand::new(cfg, root_path))
            }
            cli::Commands::ClearCache => {
                Box::new(command::ClearCacheCommand::new(cfg))
            }
            cli::Commands::DebugCache { filter, limit, show_meta } => {
                if show_meta {
                    Box::new(command::DebugCacheMetaCommand::new(cfg, filter, limit))
                } else {
                    Box::new(command::DebugCacheCommand::new(cfg, filter, limit))
                }
            }
        };
        cmd.execute().await?;
    } else {
        cli::Cli::command().print_help()?;
    }

    Ok(())
}
