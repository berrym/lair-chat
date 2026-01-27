//! # Lair Chat TUI Client
//!
//! A terminal-based chat client for Lair Chat servers.

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use clap::Parser;
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod components;
mod crypto;
mod protocol;

use app::{Action, App, Screen};
use components::{ChatRenderContext, ChatScreen, LoginScreen, RoomsScreen};

/// Lair Chat TUI Client
#[derive(Parser, Debug)]
#[command(name = "lair-chat-client")]
#[command(about = "A terminal-based chat client for Lair Chat servers")]
#[command(version)]
struct Args {
    /// Server address
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    server: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,lair_chat_client=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    // Parse arguments
    let args = Args::parse();
    let server_addr: SocketAddr = args.server.parse().expect("Invalid server address");

    info!("Lair Chat Client starting...");
    info!("Server: {}", server_addr);

    // Run the TUI
    run_tui(server_addr).await
}

async fn run_tui(server_addr: SocketAddr) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(server_addr);
    let mut login_screen = LoginScreen::new();
    let mut chat_screen = ChatScreen::new();
    let mut rooms_screen = RoomsScreen::new();

    // Connect to server
    if let Err(e) = app.connect().await {
        app.error = Some(format!("Failed to connect: {}", e));
    }

    // Main loop
    let tick_rate = Duration::from_millis(100);
    let mut last_ping = std::time::Instant::now();

    loop {
        // Draw
        terminal.draw(|frame| {
            let area = frame.area();

            match app.screen {
                Screen::Login => {
                    login_screen.render(frame, area, app.error.as_deref());
                }
                Screen::Chat => {
                    // Get online and offline user lists
                    let (online_usernames, offline_usernames) = app.get_user_lists();

                    let ctx = ChatRenderContext {
                        messages: &app.messages,
                        room_name: app.current_room.as_ref().map(|r| r.name.as_str()),
                        dm_user: app.current_dm_user.as_ref().map(|u| u.username.as_str()),
                        username: app.user.as_ref().map(|u| u.username.as_str()),
                        status: app.status.as_deref(),
                        error: app.error.as_deref(),
                        online_users: &online_usernames,
                        offline_users: &offline_usernames,
                    };
                    chat_screen.render(frame, area, &ctx);
                }
                Screen::Rooms => {
                    rooms_screen.render(
                        frame,
                        area,
                        &app.rooms,
                        app.current_room.as_ref(),
                        app.status.as_deref(),
                        app.error.as_deref(),
                    );
                }
            }
        })?;

        // Poll for events
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                // Only handle key press events (not release)
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Handle Ctrl+C globally
                if key.code == KeyCode::Char('c')
                    && key.modifiers.contains(event::KeyModifiers::CONTROL)
                {
                    app.handle_action(Action::Quit).await;
                    break;
                }

                // Route to appropriate screen
                let action = match app.screen {
                    Screen::Login => login_screen.handle_key(key),
                    Screen::Chat => chat_screen.handle_key(key, app.all_users.len()),
                    Screen::Rooms => {
                        // Debug: show which screen we're on and the key pressed
                        app.status =
                            Some(format!("Rooms: {:?}, {} rooms", key.code, app.rooms.len()));
                        let (action, debug_msg) = rooms_screen.handle_key(key, &app.rooms);
                        if let Some(msg) = debug_msg {
                            app.status = Some(msg);
                        }
                        action
                    }
                };

                if let Some(action) = action {
                    app.handle_action(action).await;
                }
            }
        }

        // Poll for server messages
        app.poll_messages().await;

        // Send keepalive ping every 30 seconds
        if last_ping.elapsed() > Duration::from_secs(30) {
            app.send_ping().await;
            last_ping = std::time::Instant::now();
        }

        // Check for quit
        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    info!("Goodbye!");
    Ok(())
}
