# The Lair Chat

## Current Version
0.3.0

## Description

An async enctypted chat app written in the Rust programming language.
It is both a terminal based server and TUI client.

## Getting started

Install a recent version of Rust using your OS distributions package manager or Rust's own preferred rustup.  For details check with your OS distribution or visit https://rust-lang.org for more information.

### Installing

Clone the git repository from https://github.com/berrym/lair-char.git

### Building and Executing The Lair

Use Rust's own tooling to compile and run the program, e.g.

* cargo build
    * Build the program
* cargo run
    * cargo run --bin lair-chat-server
    * cargo run --bin lair-chat-client

## Help

* To specify the address and port the server binds to, run the command, e.g. cargo run --bin lair-chat-server -- 127.0.0.1:8080
* To connect to the server run the client:
    * Press / to enter input mode, enter a formatted server string e.g. 127.0.0.1:8080
    * Press enter to connect and stay in input mode, or,
    * Press esc to enter command mode and press c to connect to server, then,
    * Press / to enter input mode and type messages for the server, press enter to send
* When in command mode you can press d to disconnect from the server or q to exit the client

## Authors

Copyright (c) 2024 Michael Berry <trismegustis@gmail.com>

## Version History

## License

This project is licensed under the MIT License - see the LICENSE file for details.
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
