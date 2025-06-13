use color_eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    layout::{Rect, Size},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, config::Config, tui::Event};

#[path = "components/app.rs"]
pub mod app;
#[path = "components/auth/mod.rs"]
pub mod auth;
#[path = "components/chat/mod.rs"]
pub mod chat;
#[path = "components/dm_conversation.rs"]
pub mod dm_conversation;
#[path = "components/dm_navigation.rs"]
pub mod dm_navigation;
#[path = "components/fps.rs"]
pub mod fps;
#[path = "components/home.rs"]
pub mod home;
#[path = "components/status/mod.rs"]
pub mod status;
#[path = "components/user_list.rs"]
pub mod user_list;

pub use self::auth::{AuthStatusBar, LoginScreen};
pub use self::chat::ChatView;
pub use self::dm_conversation::{ConversationEvent, ConversationPanel, ConversationState};
pub use self::dm_navigation::{NavigationEvent, NavigationPanel, NavigationState};
pub use self::status::StatusBar;
pub use self::user_list::{UserListEvent, UserListPanel, UserListState};

/// `Component` is a trait that represents a visual and interactive element of the user interface.
///
/// Implementors of this trait can be registered with the main application loop and will be able to
/// receive events, update state, and be rendered on the screen.
pub trait Component {
    /// Register an action handler that can send actions for processing if necessary.
    ///
    /// # Arguments
    ///
    /// * `tx` - An unbounded sender that can send actions.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        let _ = tx; // to appease clippy
        Ok(())
    }
    /// Register a configuration handler that provides configuration settings if necessary.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration settings.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        let _ = config; // to appease clippy
        Ok(())
    }
    /// Initialize the component with a specified area if necessary.
    ///
    /// # Arguments
    ///
    /// * `area` - Rectangular area to initialize the component within.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn init(&mut self, area: Size) -> Result<()> {
        let _ = area; // to appease clippy
        Ok(())
    }
    /// Handle incoming events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `event` - An optional event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
        let action = match event {
            Some(Event::Key(key_event)) => self.handle_key_event(key_event)?,
            Some(Event::Mouse(mouse_event)) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };
        Ok(action)
    }
    /// Handle key events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `key` - A key event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        Ok(self.handle_key(key))
    }
    /// Handle mouse events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `mouse` - A mouse event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        let _ = mouse; // to appease clippy
        Ok(None)
    }
    /// Update the state of the component based on a received action. (REQUIRED)
    ///
    /// # Arguments
    ///
    /// * `action` - An action that may modify the state of the component.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let _ = action; // to appease clippy
        Ok(None)
    }
    /// Render the component on the screen. (REQUIRED)
    ///
    /// # Arguments
    ///
    /// * `f` - A frame used for rendering.
    /// * `area` - The area in which the component should be drawn.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;

    /// Handle a key event directly
    ///
    /// # Arguments
    ///
    /// * `key` - The key event to handle
    ///
    /// # Returns
    ///
    /// * `Option<Action>` - An optional action to perform
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        let _ = key;
        None
    }
}
