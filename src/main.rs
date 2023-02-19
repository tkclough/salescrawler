mod auth;
mod error;
mod reddit;
mod models;
mod rule;
mod sms;
mod discord;
mod db;
mod poll;
mod config;
use error::Error;

use clap::{Parser, Subcommand, CommandFactory};

use crate::poll::polling_loop;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Setup,
    Poll,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let cli = Args::parse();
    println!("{:?}", &cli.command);

    match &cli.command {
        Some(Commands::Setup) => {
            let config = config::Config::read_from_toml_file("config.toml")?;
            db::Client::new(config.db).setup().await?;
        },
        Some(Commands::Poll) => {
            let config = config::Config::read_from_toml_file("config.toml")?;
            polling_loop(config).await?;
        }
        _ => {
            Args::command().print_help()?;
        }
    }

    Ok(())
}

