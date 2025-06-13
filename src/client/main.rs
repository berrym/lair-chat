use clap::Parser;
use color_eyre::Result;
use lair_chat::{app::App, cli::Cli, errors, logging};

#[tokio::main]
async fn main() -> Result<()> {
    errors::init()?;
    logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(args.tick_rate, args.frame_rate, args.text_only)?;
    app.run().await?;
    Ok(())
}
