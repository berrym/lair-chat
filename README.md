# The Lair Chat

## Current Version
0.5.9

## Description

An asynchronous encrypted chat app written in the Rust programming language.
It is both a terminal based server and TUI client.

## Modern Architecture Implementation

**Completed:**
- Implemented ConnectionManager with async/await pattern
- Created robust observer pattern for event handling
- Established comprehensive error handling with error type hierarchy
- Developed encryption abstraction with secure implementation
- Implemented proper dependency injection pattern
- Created transport abstraction layer with TCP implementation
- Built comprehensive testing infrastructure with mocks
- Eliminated global state throughout the application
- Added concurrent connection support with proper resource management
- Implemented proper authentication flow with error handling

**In Progress:**
- End-to-end testing of complete message flow
- Performance optimization and benchmarking
- Documentation updates for v0.6.0 release

See `LEGACY_MIGRATION_ACTION_PLAN.md` for migration progress and `NEXT_STEPS.md` for future plans.

## Getting started

Install the mose recent version of Rust using your OS distributions package manager or Rust's own preferred rustup.  For details check with your OS distribution or visit https://rust-lang.org for more information.

## Architecture Overview

The application now uses a modern, modular architecture with the following key components:

- **ConnectionManager**: Central coordinator that manages the lifecycle of connections
- **Transport Interface**: Abstraction for network communication
- **EncryptionService**: Interface for secure message encryption
- **ConnectionObserver**: Pattern for UI notifications and updates
- **MessageStore**: Structured message handling and formatting

This architecture provides improved testability, maintainability, and performance while eliminating global state.

### Building and Executing The Lair

Clone the git repository

    $ git clone https://github.com/berrym/lair-chat.git

Use Rust's own tooling to compile and run the program, e.g.

Build the app

    $ cargo build

Run the app

    $ cargo run --bin lair-chat-server
    $ cargo run --bin lair-chat-client

## Help

To specify the address and port the server binds to, run the command, e.g.

    $ cargo run --bin lair-chat-server -- 127.0.0.1:8080

To connect to the server run the client
-   Press / to enter input mode, enter a formatted server string e.g. 127.0.0.1:8080
-   Press enter to connect and stay in input mode, or,
-   Press esc to enter command mode and press c to connect to server, then,
-   Press / to enter input mode and type messages for the server, press enter to send
-   In Normal mode press d to disconnect from the server or q to exit the client

## Authors

Copyright (c) 2025 Michael Berry <trismegustis@gmail.com>

## License

This project is licensed under the MIT License - see the LICENSE file for details.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
