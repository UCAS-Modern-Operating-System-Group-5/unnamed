mod error;
mod config;
mod cli;
mod command;
mod constants;

use error::WrapErr;

use clap::CommandFactory;
use clap::Parser;

#[tokio::main]
async fn main() -> error::Result<()> {
    color_eyre::install()?;
    
    let cfg = config::Config::load().context("Load configuration error")?;
    let command_line = cli::Cli::parse();
    
    if let Some(command) = command_line.command {
        let cmd: Box<dyn command::Command> = match command {
            cli::Commands::Serve => {
                Box::new(command::ServeCommand::new(cfg))
            }
        };
        cmd.execute().await?;
    } else {
        cli::Cli::command().print_help()?;
    }


    Ok(())
}

