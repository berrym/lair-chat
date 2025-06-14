//! Lair-Chat Client Library
//! This library exposes all client logic for use by binaries and integration tests.

pub mod action;
pub mod aes_gcm_encryption;
pub mod app;
pub mod auth;
pub mod cli;

pub mod components;
pub mod config;
pub mod connection_manager;
pub mod encryption;
pub mod errors;
pub mod history;
pub mod logging;

pub mod tcp_transport;
pub mod transport;
pub mod tui;
