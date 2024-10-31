mod keylogger;

use clap::Parser;
use keylogger::{run_keylogger, KeyLoggerArguments};

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

fn main() {
    let args = AppArguments::parse();

    match args.subcommand {
        AppSubCommand::KeyLogger(subcommand_args) => run_keylogger(subcommand_args),
    }
}
