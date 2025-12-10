mod error;
mod settings;
mod cli;
mod command;
mod constants;

use clap::CommandFactory;
use clap::Parser;

#[tokio::main]
async fn main() -> error::Result<()> {
    color_eyre::install()?;
    
    let config = settings::Settings::from_file_or_env(None, constants::ENV_PREFIX)?;
    let command_line = cli::Cli::parse();
    
    if let Some(command) = command_line.command {
        let cmd: Box<dyn command::Command> = match command {
            cli::Commands::Serve => {
                Box::new(command::ServeCommand::new(config))
            }
        };
        cmd.execute().await?;
    } else {
        cli::Cli::command().print_help()?;
    }


    Ok(())
}

