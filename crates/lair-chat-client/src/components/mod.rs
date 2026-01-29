//! TUI components.

pub mod chat;
pub mod command_palette;
pub mod dialog;
pub mod login;
pub mod rooms;
pub mod status;
pub mod toast;

pub use chat::{ChatRenderContext, ChatScreen};
pub use command_palette::CommandPalette;
pub use dialog::{Dialog, DialogResult};
pub use login::LoginScreen;
pub use rooms::RoomsScreen;
pub use toast::render_toasts_default;
