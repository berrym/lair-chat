use color_eyre::eyre::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use std::net::SocketAddr;
use tokio::sync::mpsc;

use crate::{
    action::Action,
    config::Config,
    tui::{Event, Frame},
};

/// Shorthand for the transmit half of the message channel
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receiving half of the message channel
pub type Rx<T> = mpsc::UnboundedReceiver<T>;

pub mod home;

pub trait Component {
    #[allow(unused_variables)]
    fn register_action_handler(&mut self, tx: Tx<Action>) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn init(&mut self, area: Rect) -> Result<()> {
        Ok(())
    }

    fn get_server_address(&mut self) -> Option<SocketAddr> {
        None
    }

    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
        let r = match event {
            Some(Event::Key(key_event)) => self.handle_key_events(key_event)?,
            Some(Event::Mouse(mouse_event)) => self.handle_mouse_events(mouse_event)?,
            _ => None,
        };
        Ok(r)
    }

    #[allow(unused_variables)]
    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    fn handle_mouse_events(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        Ok(None)
    }

    #[allow(unused_variables)]
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()>;
}
