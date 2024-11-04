mod keylogger;

use clap::Parser;
use keylogger::{keylogger, KeyLoggerArguments};

#[derive(Parser, Debug)]
#[clap(author = "sysraccoon", version, about)]
struct AppArguments {
    #[clap(subcommand)]
    subcommand: AppSubCommand,
}

#[derive(Parser, Debug)]
enum AppSubCommand {
    KeyLogger(KeyLoggerArguments),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = AppArguments::parse();

    match args.subcommand {
        AppSubCommand::KeyLogger(subcommand_args) => keylogger(subcommand_args)?,
    };

    Ok(())
}
