use clap::Parser;
use cli::Cli;
use color_eyre::Result;

use crate::app::App;

mod action;
mod aes_gcm_encryption;
mod app;
mod cli;
mod components;
mod config;
mod connection_manager;
mod encryption;
mod errors;
mod logging;
mod tcp_transport;
mod transport;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate)?;
    app.run().await?;
    Ok(())
}
