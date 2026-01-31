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

use app::{Action, App, Screen, TransportType};
use components::{
    render_toasts_default, ChatRenderContext, ChatScreen, CommandPalette, Dialog, DialogResult,
    HelpOverlay, InvitationAction, InvitationsOverlay, LoginScreen, MemberAction, MembersOverlay,
    RoomsScreen,
};

/// Lair Chat TUI Client
#[derive(Parser, Debug)]
#[command(name = "lair-chat-client")]
#[command(about = "A terminal-based chat client for Lair Chat servers")]
#[command(version)]
struct Args {
    /// TCP server address (for real-time messaging)
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    server: String,

    /// HTTP API base URL (http:// or https://).
    /// If not specified, derived from TCP server address.
    #[arg(long)]
    http_url: Option<String>,

    /// Skip TLS certificate verification (for development with self-signed certs).
    /// WARNING: Do not use in production!
    #[arg(long, default_value = "false")]
    insecure: bool,

    /// Use WebSocket instead of TCP for real-time messaging.
    /// WebSocket works through HTTP proxies and on HTTP-only networks.
    /// Note: WebSocket uses TLS for encryption (no E2E encryption).
    #[arg(long, short = 'w')]
    websocket: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Initialize logging - write to stderr to avoid interfering with TUI
    // Default to warnings only; use RUST_LOG=lair_chat_client=info for verbose output
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_writer(std::io::stderr),
        )
        .init();

    // Parse arguments
    let args = Args::parse();
    let server_addr: SocketAddr = args.server.parse().expect("Invalid server address");

    // Derive HTTP URL from server address if not specified
    let http_url = args
        .http_url
        .unwrap_or_else(|| format!("http://{}:{}", server_addr.ip(), server_addr.port() + 2));

    // Determine transport type
    let transport_type = if args.websocket {
        TransportType::WebSocket
    } else {
        TransportType::Tcp
    };

    info!("Lair Chat Client starting...");
    info!("Server: {}", server_addr);
    info!("HTTP URL: {}", http_url);
    info!("Transport: {}", transport_type);
    if args.insecure {
        info!("TLS verification: disabled (insecure mode)");
    }

    // Run the TUI
    run_tui(server_addr, http_url, args.insecure, transport_type).await
}

async fn run_tui(
    server_addr: SocketAddr,
    http_url: String,
    insecure: bool,
    transport_type: TransportType,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app with HTTP configuration and transport type
    let mut app = App::with_transport(server_addr, http_url, insecure, transport_type);

    // Create login screen with the server address from CLI (or default)
    let mut login_screen = LoginScreen::with_server(server_addr.to_string());

    let mut chat_screen = ChatScreen::new();
    let mut rooms_screen = RoomsScreen::new();
    let mut command_palette = CommandPalette::new();
    let mut dialog = Dialog::new();
    let mut help_overlay = HelpOverlay::new();
    let mut invitations_overlay = InvitationsOverlay::new();
    let mut members_overlay = MembersOverlay::new();

    // Note: Connection is now deferred until user submits login form
    // This allows the user to configure the server address in the UI

    // Main loop
    let tick_rate = Duration::from_millis(100);
    let mut last_ping = std::time::Instant::now();
    let mut last_screen = app.screen.clone();

    loop {
        // Check for screen changes and force terminal clear if needed
        if app.screen != last_screen {
            terminal.clear()?;
            last_screen = app.screen.clone();
        }

        // Draw
        terminal.draw(|frame| {
            let area = frame.area();

            match app.screen {
                Screen::Login => {
                    login_screen.render(frame, area, app.error());
                }
                Screen::Chat => {
                    // Get online and offline user lists
                    let (online_usernames, offline_usernames) = app.get_user_lists();
                    let unread_dms = app.get_unread_dms();

                    let ctx = ChatRenderContext {
                        messages: &app.messages,
                        room_name: app.current_room.as_ref().map(|r| r.name.as_str()),
                        dm_user: app.current_dm_user.as_ref().map(|u| u.username.as_str()),
                        username: app.user.as_ref().map(|u| u.username.as_str()),
                        status: app.status.as_deref(),
                        error: app.error(),
                        online_users: &online_usernames,
                        offline_users: &offline_usernames,
                        unread_dms: &unread_dms,
                        pending_invitation_count: app.pending_invitations.len(),
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
                        app.error(),
                    );
                }
            }

            // Render command palette as overlay (if visible)
            command_palette.render(frame, area);

            // Render dialog as overlay
            dialog.render(frame, area);

            // Render invitations overlay
            invitations_overlay.render(frame, area, &app.pending_invitations);

            // Render members overlay
            members_overlay.render(frame, area, &app.current_room_members);

            // Render help overlay
            help_overlay.render(frame, area);

            // Render toast notifications last (topmost, always visible)
            let notifications = app.notifications();
            render_toasts_default(frame, area, &notifications);
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

                // Route to help overlay if visible (highest priority)
                if help_overlay.visible {
                    help_overlay.handle_key(key);
                    continue;
                }

                // Route to dialog if visible
                if dialog.visible {
                    match dialog.handle_key(key) {
                        DialogResult::Confirmed(_) => {
                            // Dialog was confirmed - check if it was quit confirmation
                            if dialog.title.contains("Quit") {
                                app.handle_action(Action::Quit).await;
                                break;
                            }
                            // Check if it was invite confirmation
                            if dialog.title.contains("Invite") {
                                // Invite was confirmed - the action has already been handled
                            }
                        }
                        DialogResult::Cancelled => {
                            // Dialog was cancelled, do nothing
                        }
                        DialogResult::Pending => {
                            // Dialog still open
                        }
                    }
                    continue;
                }

                // Route to invitations overlay if visible
                if invitations_overlay.visible {
                    if let Some(action) =
                        invitations_overlay.handle_key(key, app.pending_invitations.len())
                    {
                        match action {
                            InvitationAction::Accept => {
                                // Get the actual invitation ID from the selected index
                                if let Some(idx) = invitations_overlay.selected_index() {
                                    if let Some(inv) = app.pending_invitations.get(idx) {
                                        let inv_id = inv.id;
                                        let context_changed = app
                                            .handle_action(Action::AcceptInvitation(inv_id))
                                            .await;
                                        if context_changed {
                                            chat_screen.scroll_to_bottom();
                                            invitations_overlay.hide();
                                        }
                                    }
                                }
                            }
                            InvitationAction::Decline => {
                                // Get the actual invitation ID from the selected index
                                if let Some(idx) = invitations_overlay.selected_index() {
                                    if let Some(inv) = app.pending_invitations.get(idx) {
                                        let inv_id = inv.id;
                                        app.handle_action(Action::DeclineInvitation(inv_id)).await;
                                    }
                                }
                            }
                            InvitationAction::Close => {
                                // Already handled by handle_key
                            }
                            InvitationAction::Refresh => {
                                app.handle_action(Action::RefreshInvitations).await;
                            }
                        }
                    }
                    continue;
                }

                // Route to members overlay if visible
                if members_overlay.visible {
                    if let Some(action) =
                        members_overlay.handle_key(key, app.current_room_members.len())
                    {
                        match action {
                            MemberAction::Close => {
                                // Already handled by handle_key
                            }
                            MemberAction::StartDM => {
                                // Get the actual user from the selected index
                                if let Some(idx) = members_overlay.selected_index() {
                                    if let Some(member) = app.current_room_members.get(idx) {
                                        let username = member.username.clone();
                                        members_overlay.hide();
                                        let context_changed =
                                            app.handle_action(Action::StartDM(username)).await;
                                        if context_changed {
                                            chat_screen.scroll_to_bottom();
                                        }
                                    }
                                }
                            }
                            MemberAction::KickMember => {
                                // Kick the selected member
                                if let Some(idx) = members_overlay.selected_index() {
                                    if let Some(member) = app.current_room_members.get(idx) {
                                        let user_id = member.user_id;
                                        app.handle_action(Action::KickMember { user_id }).await;
                                    }
                                }
                            }
                            MemberAction::PromoteToMod => {
                                // Promote selected member to moderator
                                if let Some(idx) = members_overlay.selected_index() {
                                    if let Some(member) = app.current_room_members.get(idx) {
                                        let user_id = member.user_id;
                                        app.handle_action(Action::ChangeMemberRole {
                                            user_id,
                                            role: "moderator".to_string(),
                                        })
                                        .await;
                                    }
                                }
                            }
                            MemberAction::DemoteToMember => {
                                // Demote selected member to regular member
                                if let Some(idx) = members_overlay.selected_index() {
                                    if let Some(member) = app.current_room_members.get(idx) {
                                        let user_id = member.user_id;
                                        app.handle_action(Action::ChangeMemberRole {
                                            user_id,
                                            role: "member".to_string(),
                                        })
                                        .await;
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }

                // Handle Ctrl+P to toggle command palette
                if key.code == KeyCode::Char('p')
                    && key.modifiers.contains(event::KeyModifiers::CONTROL)
                {
                    command_palette.toggle();
                    continue;
                }

                // Route to command palette if visible
                if command_palette.visible {
                    if let Some(action) = command_palette.handle_key(key) {
                        // Handle special actions from command palette
                        match &action {
                            Action::CopyLastMessage => {
                                if let Some(content) = app.last_message_content() {
                                    chat_screen.copy_to_clipboard(content);
                                    app.set_info("Copied to clipboard");
                                }
                            }
                            Action::Quit => {
                                dialog.show_confirm("Quit", "Are you sure you want to quit?");
                            }
                            Action::ShowHelp => {
                                help_overlay.show();
                            }
                            _ => {
                                let context_changed = app.handle_action(action).await;
                                if context_changed {
                                    chat_screen.scroll_to_bottom();
                                }
                            }
                        }
                    }
                    continue;
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
                    // Handle special actions
                    match &action {
                        Action::CopyLastMessage => {
                            if let Some(content) = app.last_message_content() {
                                chat_screen.copy_to_clipboard(content);
                                app.set_info("Copied to clipboard");
                            }
                        }
                        Action::Quit => {
                            // Show quit confirmation dialog
                            dialog.show_confirm("Quit", "Are you sure you want to quit?");
                        }
                        Action::ShowHelp => {
                            // Show help in a popup dialog
                            help_overlay.show();
                        }
                        Action::ShowInvitations => {
                            // Fetch invitations and show overlay
                            app.handle_action(Action::RefreshInvitations).await;
                            invitations_overlay.show(app.pending_invitations.len());
                        }
                        Action::ShowMembers => {
                            if let Some(room) = &app.current_room {
                                let room_name = room.name.clone();
                                app.handle_action(Action::ShowMembers).await;
                                members_overlay.show(&room_name, app.current_room_members.len());
                            } else {
                                app.set_error("No room selected");
                            }
                        }
                        Action::InviteUser {
                            user_id,
                            room_id,
                            message,
                        } => {
                            app.handle_action(Action::InviteUser {
                                user_id: *user_id,
                                room_id: *room_id,
                                message: message.clone(),
                            })
                            .await;
                        }
                        _ => {
                            let context_changed = app.handle_action(action).await;
                            if context_changed {
                                chat_screen.scroll_to_bottom();
                            }
                        }
                    }
                }
            }
        }

        // Poll for server messages (smart scroll: auto-scroll if user was at bottom)
        let new_messages = app.poll_messages().await;
        if new_messages && chat_screen.is_at_bottom() {
            chat_screen.scroll_to_bottom();
        }

        // Tick notifications to auto-dismiss expired ones
        app.tick_notifications();

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
