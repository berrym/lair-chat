# Lair Chat TUI Client

A terminal-based chat client for Lair Chat servers, built with Ratatui.

## Features

- **Terminal UI**: Full-featured TUI with login, chat, and room management screens
- **Real-time messaging**: Instant message delivery via TCP connection
- **HTTP authentication**: Login/register via REST API per [ADR-013](../../docs/architecture/DECISIONS.md#adr-013-protocol-responsibility-split)
- **Direct messages**: Private one-on-one conversations with `/dm <username>`
- **Message history**: Automatically loads recent messages when joining a room or DM
- **Room support**: Create, join, and switch between chat rooms
- **Room invitations**: Invite users to private rooms, accept/decline invitations with `I` key
- **Room member management**: View members, change roles, kick users (owner/moderator)
- **Online users panel**: See who's in the current room with a sidebar
- **User presence**: See when users come online or go offline
- **Vim-like navigation**: Use j/k, G/g for scrolling through messages
- **Auto-reconnect**: Automatic reconnection when connection is lost
- **Keepalive**: Automatic connection maintenance with ping/pong

## Installation

### From Source

```bash
# From the workspace root
cargo build --release -p lair-chat-client

# Binary will be at target/release/lair-chat-client
```

### Running

```bash
# Connect to default server (127.0.0.1 - HTTP:8082, TCP:8080)
lair-chat-client

# Connect to specific server
lair-chat-client --server 192.168.1.100:8080

# Or using short flag
lair-chat-client -s 192.168.1.100:8080
```

## Usage

### Login Screen

When the client starts, you'll see the login screen. The server address field is
pre-filled from the `--server` CLI argument (default: `127.0.0.1:8080`) and can
be edited before logging in:

```
┌─────────────────────────────────────────┐
│            LAIR CHAT                    │
│                                         │
│  Server:   [127.0.0.1:8080_____]        │
│  Username: [____________]               │
│  Password: [____________]               │
│  Email:    [____________] (register)    │
│                                         │
│  [Tab] Switch fields                    │
│  [Enter] Login / [F1] Register mode     │
│  [Esc] Quit                             │
└─────────────────────────────────────────┘
```

**Controls:**
- `Tab` - Move between input fields
- `Enter` - Login with current credentials (connects to server first)
- `F1` - Toggle between login and registration mode
- `Esc` - Quit

### Chat Screen

After logging in, you'll see the main chat interface:

```
┌─────────────────────────────────────────┐
│ #general                    user: alice │
├─────────────────────────────────────────┤
│ [System] Connected to server            │
│ [System] Login successful!              │
│ [System] Joined room: general           │
│ bob: Hello everyone!                    │
│ alice: Hi bob!                          │
│                                         │
├─────────────────────────────────────────┤
│ > [type your message here...]           │
└─────────────────────────────────────────┘
```

**Controls (Normal mode):**
- `i` - Enter insert mode to type a message
- `q` - Quit the application
- `r` - Open room list
- `I` - Show pending invitations
- `m` - Show room members (when in a room)
- `j`/`↓` - Scroll down through messages
- `k`/`↑` - Scroll up through messages
- `G` - Jump to newest message
- `g` - Jump to oldest message
- `Tab` - Switch focus between messages and users panel
- `?`/`F1` - Show help overlay

**Controls (Normal mode continued):**
- `R` - Reconnect to server (if disconnected)
- `Ctrl+P` - Open command palette

**Controls (Insert mode):**
- Type your message
- `Enter` - Send message
- `Esc` - Return to normal mode
- `/quit` - Quit the application
- `/rooms` - Open room list
- `/create <name>` - Create a new room
- `/dm <username>` - Start a direct message with a user

### Direct Messages

You can send private messages to other users:

1. Use the `/dm <username>` command in insert mode
2. The chat title changes to "DM: username" with a magenta border
3. Message history with that user is loaded
4. Send messages normally - they go directly to that user
5. Use `/rooms` or `r` to return to room chat

When you receive a DM while viewing a room, you'll see a notification:
"New DM from username"

### Rooms Screen

Press `r` (in normal mode) from chat to see available rooms:

```
┌─────────────────────────────────────────┐
│              ROOMS                      │
├─────────────────────────────────────────┤
│ > general (5 members)                   │
│   random (3 members)                    │
│   dev-chat (12 members)                 │
│                                         │
│  [↑/↓] Navigate  [Enter] Join           │
│  [N] New room    [Esc] Back             │
└─────────────────────────────────────────┘
```

**Controls:**
- `j`/`↓` - Move selection down
- `k`/`↑` - Move selection up
- `Enter` - Join selected room
- `n` - Create new room (prompts for name)
- `Esc` - Return to chat
- `q` - Quit the application

### Invitations Overlay

Press `I` (Shift+i) to view pending room invitations:

```
┌─────────────────────────────────────────┐
│           INVITATIONS (2)               │
├─────────────────────────────────────────┤
│ > #dev-team from alice                  │
│   #random from bob                      │
│                                         │
│  [↑/↓] Navigate  [Enter] Accept         │
│  [d] Decline     [Esc] Close            │
└─────────────────────────────────────────┘
```

**Controls:**
- `j`/`↓` - Move selection down
- `k`/`↑` - Move selection up
- `Enter` - Accept invitation and join room
- `d` - Decline invitation
- `Esc` - Close overlay

When you receive an invitation while online, a notification appears and the
status bar shows a badge like `[2]` indicating pending invitation count.

### Room Members Overlay

Press `m` while in a room to view and manage members:

```
┌─────────────────────────────────────────┐
│        MEMBERS: #general (5)            │
├─────────────────────────────────────────┤
│ > alice [owner] ●                       │
│   bob [moderator] ●                     │
│   charlie [member] ○                    │
│                                         │
│  [↑/↓] Navigate  [r] Change role        │
│  [k] Kick        [Esc] Close            │
└─────────────────────────────────────────┘
```

**Controls:**
- `j`/`↓` - Move selection down
- `k`/`↑` - Move selection up
- `r` - Change member's role (owner only)
- `k` - Kick member from room (owner/moderator)
- `Esc` - Close overlay

**Permission Rules:**
- Owners can change roles and kick anyone except themselves
- Moderators can kick regular members only
- Regular members can only view the member list

### Users Panel

Press `Tab` to switch focus to the users panel on the right side of the chat:

**Controls (when focused on users panel):**
- `j`/`k` - Navigate user list
- `Enter` - Start DM with selected user
- `i` - Invite selected user to current room (owner/moderator only)
- `Tab`/`Esc` - Return to messages

## Architecture

The client implements ADR-013's protocol responsibility split:

1. **HTTP** (port 8082) - Authentication (login/register) and queries
2. **TCP** (port 8080) - Real-time messaging and events

```
┌────────┐                  ┌──────────┐                  ┌────────┐
│ Client │                  │   HTTP   │                  │  TCP   │
└───┬────┘                  └────┬─────┘                  └───┬────┘
    │                            │                            │
    │── POST /auth/login ───────▶│                            │
    │◀── JWT Token + User ───────│                            │
    │                            │                            │
    │─────────────── TCP Connect ────────────────────────────▶│
    │◀────────────── ServerHello ─────────────────────────────│
    │─────────────── ClientHello ────────────────────────────▶│
    │─────────────── Authenticate(jwt) ──────────────────────▶│
    │◀────────────── AuthenticateResponse ────────────────────│
    │                            │                            │
    │◀═══════════════ Real-time Messaging ═══════════════════▶│
```

### Directory Structure

```
src/
├── main.rs           # Entry point, terminal setup, event loop
├── app.rs            # Application state and business logic
├── protocol/         # Protocol implementations
│   ├── mod.rs
│   ├── tcp.rs        # TCP client with framing
│   ├── http.rs       # HTTP client for auth and API (per ADR-013)
│   └── messages.rs   # Protocol message types
└── components/       # TUI components
    ├── mod.rs
    ├── login.rs      # Login/register screen
    ├── chat.rs       # Main chat view
    ├── rooms.rs      # Room list/selection
    ├── invitations.rs # Invitations overlay
    ├── members.rs    # Room members overlay
    ├── help.rs       # Full-screen help overlay
    ├── command_palette.rs # Command palette overlay
    └── status.rs     # Status bar utilities
```

### Protocol Layer

The client implements the Lair Chat protocols as documented:

- **[HTTP API](../../docs/protocols/HTTP.md)** - Authentication and CRUD
- **[TCP Protocol](../../docs/protocols/TCP.md)** - Real-time messaging

Messages are length-prefixed JSON over TCP:

```
┌──────────────┬─────────────────────────────────┐
│ Length (4B)  │ JSON Payload                    │
│ Big-endian   │                                 │
└──────────────┴─────────────────────────────────┘
```

The `Connection` type manages the TCP connection with separate reader/writer
tasks for non-blocking I/O.

## Configuration

### Environment Variables

- `RUST_LOG` - Control logging verbosity (default: `warn`)
  - Use `RUST_LOG=lair_chat_client=info` for verbose client logging
  - Use `RUST_LOG=lair_chat_client=debug` for debug output

### Command-Line Options

```
Usage: lair-chat-client [OPTIONS]

Options:
  -s, --server <SERVER>  Server address [default: 127.0.0.1:8080]
  -h, --help             Print help
  -V, --version          Print version
```

Note: The HTTP port is automatically inferred as TCP port + 2 (e.g., TCP 8080 -> HTTP 8082).

## Troubleshooting

### Connection Refused

Make sure the Lair Chat server is running:

```bash
# From workspace root
cargo run -p lair-chat-server
```

The server must expose both:
- HTTP on port 8082 (for authentication)
- TCP on port 8080 (for real-time messaging)

### Display Issues

If the terminal display looks broken:
1. Ensure your terminal supports 256 colors
2. Try resizing the terminal window
3. Check that your terminal supports Unicode

### Logging

Enable verbose logging to see what's happening:

```bash
# Info-level logging (recommended for troubleshooting)
RUST_LOG=lair_chat_client=info cargo run -p lair-chat-client 2>client.log

# Debug-level logging (very verbose)
RUST_LOG=lair_chat_client=debug cargo run -p lair-chat-client 2>client.log
```

Logs are written to stderr to avoid interfering with the TUI. Redirect stderr
to a file as shown above, then `tail -f client.log` in another terminal.

## Development

### Building

```bash
cargo build -p lair-chat-client
```

### Testing

```bash
cargo test -p lair-chat-client
```

### Running Against Local Server

In one terminal:
```bash
cargo run -p lair-chat-server
```

In another terminal:
```bash
cargo run -p lair-chat-client
```

## Protocol Compatibility

This client implements:
- HTTP API v1 for authentication
- TCP Protocol v1.1 for real-time messaging

It's designed to work with `lair-chat-server` but can connect to any server
implementing the same protocols as specified in the docs/protocols/ directory.

The protocol documentation is intentionally language-agnostic - you could
implement a compatible client in Python, Go, TypeScript, or any language
that supports HTTP requests, TCP sockets, and JSON.

## License

MIT
