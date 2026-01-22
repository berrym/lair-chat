# Lair Chat TUI Client

A terminal-based chat client for Lair Chat servers, built with Ratatui.

## Features

- **Terminal UI**: Full-featured TUI with login, chat, and room management screens
- **Real-time messaging**: Instant message delivery via TCP connection
- **Direct messages**: Private one-on-one conversations with `/dm <username>`
- **Message history**: Automatically loads recent messages when joining a room or DM
- **Room support**: Create, join, and switch between chat rooms
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
# Connect to default server (127.0.0.1:8080)
lair-chat-client

# Connect to specific server
lair-chat-client --server 192.168.1.100:8080

# Or using short flag
lair-chat-client -s 192.168.1.100:8080
```

## Usage

### Login Screen

When the client starts, you'll see the login screen:

```
┌─────────────────────────────────────────┐
│            LAIR CHAT                    │
│                                         │
│  Username: [____________]               │
│  Password: [____________]               │
│  Email:    [____________] (register)    │
│                                         │
│  [Tab] Switch fields                    │
│  [Enter] Login / [Ctrl+R] Register      │
│  [Ctrl+C] Quit                          │
└─────────────────────────────────────────┘
```

**Controls:**
- `Tab` - Move between input fields
- `Enter` - Login with current credentials
- `Ctrl+R` - Register new account (requires email field)
- `Ctrl+C` - Quit

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
- `j`/`↓` - Scroll down through messages
- `k`/`↑` - Scroll up through messages
- `G` - Jump to newest message
- `g` - Jump to oldest message

**Controls (Normal mode continued):**
- `R` - Reconnect to server (if disconnected)

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

## Architecture

The client is built with a clean separation of concerns:

```
src/
├── main.rs           # Entry point, terminal setup, event loop
├── app.rs            # Application state and business logic
├── protocol/         # TCP protocol implementation
│   ├── mod.rs
│   ├── tcp.rs        # TCP client with framing
│   └── messages.rs   # Protocol message types
└── components/       # TUI components
    ├── mod.rs
    ├── login.rs      # Login/register screen
    ├── chat.rs       # Main chat view
    ├── rooms.rs      # Room list/selection
    └── status.rs     # Status bar utilities
```

### Protocol Layer

The client implements the Lair Chat TCP protocol as documented in
`docs/protocols/TCP.md`. Messages are length-prefixed JSON:

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

- `RUST_LOG` - Control logging verbosity (default: `warn,lair_chat_client=info`)

### Command-Line Options

```
Usage: lair-chat-client [OPTIONS]

Options:
  -s, --server <SERVER>  Server address [default: 127.0.0.1:8080]
  -h, --help             Print help
  -V, --version          Print version
```

## Troubleshooting

### Connection Refused

Make sure the Lair Chat server is running:

```bash
# From workspace root
cargo run -p lair-chat-server
```

### Display Issues

If the terminal display looks broken:
1. Ensure your terminal supports 256 colors
2. Try resizing the terminal window
3. Check that your terminal supports Unicode

### Logging

Enable debug logging to see what's happening:

```bash
RUST_LOG=debug lair-chat-client
```

Note: In TUI mode, logs won't be visible. Run with `RUST_LOG=debug` and check
stderr output, or modify the code to log to a file.

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

This client implements the Lair Chat TCP protocol v1. It's designed to work
with `lair-chat-server` but can connect to any server implementing the same
protocol as specified in `docs/protocols/TCP.md`.

The protocol documentation is intentionally language-agnostic - you could
implement a compatible client in Python, Go, TypeScript, or any language
that supports TCP sockets and JSON.

## License

MIT
