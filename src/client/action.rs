use crate::auth::{AuthState, Credentials};
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
    Reconnect,
    // Authentication actions
    Login(Credentials),
    Register(Credentials),
    LoginWithServer(Credentials, String),
    RegisterWithServer(Credentials, String),
    Logout,
    RefreshSession,
    AuthenticationSuccess(AuthState),
    AuthenticationFailure(String),
    RegistrationSuccess(String),
    // Message actions
    SendMessage(String),
    ReceiveMessage(String),
    RecordReceivedMessage,
    RecordSentMessage,
    MessageSent(String),
    // Direct Message actions
    ToggleDM,
    // Connection status actions
    ConnectionStatusChanged(crate::transport::ConnectionStatus),
}
