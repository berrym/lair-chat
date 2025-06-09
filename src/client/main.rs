use clap::Parser;
use color_eyre::Result;
use lair_chat::{cli::Cli, app::App, errors, logging};

#[tokio::main]
async fn main() -> Result<()> {
    errors::init()?;
    logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate)?;
    app.run().await?;
    Ok(())
}
