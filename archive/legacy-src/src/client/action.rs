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
    RouteMessage(String), // Route message through message router
    RecordReceivedMessage,
    RecordSentMessage,
    MessageSent(String),
    // Direct Message actions
    ToggleDM,
    OpenDMNavigation,            // Open DM navigation from status bar
    UpdateUnreadDMCount(u32),    // Update unread DM count in status bar
    MarkAllDMsRead,              // Mark all DM conversations as read
    StartDMConversation(String), // Start DM with username
    ReturnToLobby,               // Exit DM mode and return to Lobby
    // Connection status actions
    ConnectionStatusChanged(crate::transport::ConnectionStatus),
    // Room actions
    CreateRoom(String),            // Create a new room with name
    JoinRoom(String),              // Join an existing room
    LeaveRoom,                     // Leave current room
    ListRooms,                     // List available rooms
    RoomCreated(String),           // Room creation success
    RoomJoined(String),            // Room join success
    RoomLeft(String),              // Room leave success
    RoomError(String),             // Room operation error
    RoomListReceived(Vec<String>), // List of available rooms
    CurrentRoomChanged(String),    // Current room changed
    // Invitation actions
    InvitationReceived(String, String, String), // (inviter, room_name, message)
    InvitationAccepted(String),                 // Accept invitation to room
    InvitationDeclined(String),                 // Decline invitation to room
    InviteError(String),                        // Invitation error message
    // Message Router actions
    DisplayMessage { content: String, is_system: bool }, // Display a message through the router
    UpdateConnectedUsers(Vec<String>),                   // Update list of connected users
    UpdateCurrentRoom(String),                           // Update current room in status bar
}
