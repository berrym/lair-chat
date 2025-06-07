use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    ToggleFps,
    ToggleShowHelp,
    EnterNormal,
    EnterInsert,
    EnterProcessing,
    ExitProcessing,
    ConnectClient,
    DisconnectClient,
    Update,
    ShowConnectionDialog,
    Connect(SocketAddr),
}
