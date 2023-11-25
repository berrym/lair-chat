use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures::{select, FutureExt, SinkExt};
use log::error;
use ratatui::{prelude::*, widgets::*};
use std::{collections::HashMap, net::SocketAddr, sync::Mutex};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::mpsc,
    time::Duration,
};
use tokio_stream::{wrappers::LinesStream, Stream, StreamExt};
use tokio_util::{
    codec::{FramedWrite, LinesCodec},
    sync::CancellationToken,
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
    action::Action,
    config::{key_event_to_string, Config, KeyBindings},
    mode::Mode,
    transport::*,
};

use super::{Component, Frame};

/// Shorthand for the transmit half of the message channel
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receiving half of the message channel
pub type Rx<T> = mpsc::UnboundedReceiver<T>;

/// Get any text in the input box
pub async fn get_user_input(mut input: Input) -> Option<String> {
    let message = input.value().to_string();
    input.reset();
    if message.is_empty() {
        None
    } else {
        Some(message)
    }
}

pub struct Home {
    config: Config,
    show_help: bool,
    app_ticker: usize,
    render_ticker: usize,
    mode: Mode,
    input: Input,
    action_tx: Option<Tx<Action>>,
    keymap: HashMap<KeyEvent, Action>,
    last_events: Vec<KeyEvent>,
}

impl Home {
    pub fn new() -> Home {
        Home {
            config: Config::default(),
            show_help: false,
            app_ticker: 0,
            render_ticker: 0,
            mode: Mode::Normal,
            input: Input::default(),
            action_tx: None,
            keymap: HashMap::new(),
            last_events: Vec::new(),
        }
    }

    pub fn schedule_connect_client(&mut self) {
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::EnterProcessing).unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            tx.send(Action::ConnectClient).unwrap();
            tx.send(Action::ExitProcessing).unwrap();
        });
    }

    pub fn schedule_disconnect_client(&mut self) {
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::EnterProcessing).unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            tx.send(Action::DisconnectClient).unwrap();
            tx.send(Action::ExitProcessing).unwrap();
        });
    }

    pub fn tick(&mut self) {
        log::info!("Tick");
        self.app_ticker = self.app_ticker.saturating_add(1);
        self.last_events.drain(..);
    }

    pub fn render_tick(&mut self) {
        log::debug!("Render Tick");
        self.render_ticker = self.render_ticker.saturating_add(1);
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: Tx<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        self.last_events.push(key.clone());
        let action = match self.mode {
            Mode::Normal | Mode::Processing => match key.code {
                KeyCode::Char('q') => {
                    self.schedule_disconnect_client();
                    Action::Quit
                },
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.schedule_disconnect_client();
                    Action::Quit
                },
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.schedule_disconnect_client();
                    Action::Quit
                },
                KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Suspend,
                KeyCode::Char('?') => Action::ToggleShowHelp,
                KeyCode::Char('/') => Action::EnterInsert,
                KeyCode::Char('c') => {
                    if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
                        add_text_message("Already connected to a server.".to_string());
                        return Ok(Some(Action::Update));
                    }
                    self.schedule_connect_client();
                    Action::Update
                },
                KeyCode::Char('d') => {
                    self.schedule_disconnect_client();
                    Action::Update
                },
                KeyCode::Esc => {
                    if self.show_help {
                        self.show_help = false;
                    }
                    Action::Update
                },
                _ => Action::Tick,
            },
            Mode::Insert => match key.code {
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.schedule_disconnect_client();
                    Action::Quit
                },
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.schedule_disconnect_client();
                    Action::Quit
                },
                KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Suspend,
                KeyCode::Esc => Action::EnterNormal,
                KeyCode::Enter => {
                    let client_status = CLIENT_STATUS.lock().unwrap();
                    let message = self.input.value();
                    if !message.is_empty() && client_status.status == ConnectionStatus::CONNECTED {
                        add_outgoing_message(message.to_string());
                        self.input.reset();
                    } else {
                        if !message.is_empty() {
                            add_text_message("Connect to a server to send messages.".to_string());
                        }
                        if client_status.status == ConnectionStatus::CONNECTED {
                            add_text_message("Can't send an empty message.".to_string());
                        }
                    }
                    Action::Update
                },
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    Action::Tick
                },
            },
            _ => {
                self.input.handle_event(&crossterm::event::Event::Key(key));
                Action::Tick
            },
        };
        Ok(Some(action))
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => self.tick(),
            Action::Render => self.render_tick(),
            Action::ToggleShowHelp => self.show_help = !self.show_help,
            Action::EnterNormal => {
                self.mode = Mode::Normal;
            },
            Action::EnterInsert => {
                self.mode = Mode::Insert;
            },
            Action::EnterProcessing => {
                self.mode = Mode::Processing;
            },
            Action::ExitProcessing => {
                // TODO: Make this go to previous mode instead
                self.mode = Mode::Normal;
            },
            Action::ConnectClient => {
                let user_input = self.input.value().to_string();
                self.input.reset();
                if user_input.is_empty() {
                    add_text_message("Enter a address:port string in the input box, e.g. 127.0.0.1:8080".to_string());
                    return Ok(Some(Action::Update));
                }
                let address: SocketAddr = match user_input.parse() {
                    Ok(address) => address,
                    Err(_) => {
                        add_text_message("Failed to get server address.".to_string());
                        return Ok(Some(Action::Update));
                    },
                };
                let input = self.input.clone();
                tokio::spawn(async move {
                    connect_client(input, address).await;
                });
            },
            Action::DisconnectClient => {
                tokio::spawn(async move {
                    disconnect_client().await;
                });
            },
            _ => {},
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let rects =
            Layout::default().constraints([Constraint::Percentage(100), Constraint::Min(3)].as_ref()).split(area);

        let mut text: Vec<Line> = Vec::<Line>::new();
        text.insert(0, "".into());
        let messages: Vec<Line> = MESSAGES.lock().unwrap().text.clone().iter().map(|l| Line::from(l.clone())).collect();
        if messages.is_empty() {
            text.insert(0, "No messages to display.".dim().into());
        } else {
            for l in messages {
                text.insert(0, l.into());
            }
        }
        text.insert(0, "".into());

        let height: usize = rects[0].height.into();
        let mut p: Vec<Line> = Vec::new();
        let count = text.len();
        if count > 0 {
            if count > height {
                p.append(&mut text[0..height].to_vec());
            } else {
                p.append(&mut text);
            }
            p.reverse();
            text.clear();
            text.append(&mut p.clone());
        }

        f.render_widget(
            Paragraph::new(text)
                .scroll((1, 0))
                .block(
                    Block::default()
                        .title("The Lair v0.2.0 (c) 2023 Michael Berry")
                        .title_alignment(Alignment::Center)
                        .borders(Borders::ALL)
                        .border_style(match self.mode {
                            Mode::Processing => Style::default().fg(Color::Yellow),
                            _ => Style::default(),
                        })
                        .border_type(BorderType::Rounded),
                )
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Left),
            rects[0],
        );

        let width = rects[1].width.max(3) - 3; // keep 2 for borders and 1 for cursor
        let scroll = self.input.visual_scroll(width as usize);
        let input = Paragraph::new(self.input.value())
            .style(match self.mode {
                Mode::Insert => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            })
            .scroll((0, scroll as u16))
            .block(Block::default().borders(Borders::ALL).title(Line::from(vec![
                Span::raw("Enter Input Mode "),
                Span::styled("(Press ", Style::default().fg(Color::DarkGray)),
                Span::styled("/", Style::default().add_modifier(Modifier::BOLD).fg(Color::Gray)),
                Span::styled(" to start, ", Style::default().fg(Color::DarkGray)),
                Span::styled("ESC", Style::default().add_modifier(Modifier::BOLD).fg(Color::Gray)),
                Span::styled(" to finish, ", Style::default().fg(Color::DarkGray)),
                Span::styled("?", Style::default().add_modifier(Modifier::BOLD).fg(Color::Gray)),
                Span::styled(" for help)", Style::default().fg(Color::DarkGray)),
            ])));
        f.render_widget(input, rects[1]);
        if self.mode == Mode::Insert {
            f.set_cursor(
                (rects[1].x + 1 + self.input.cursor() as u16).min(rects[1].x + rects[1].width - 2),
                rects[1].y + 1,
            )
        }

        if self.show_help {
            let rect = area.inner(&Margin { horizontal: 4, vertical: 2 });
            f.render_widget(Clear, rect);
            let block = Block::default()
                .title(Line::from(vec![Span::styled("Key Bindings", Style::default().add_modifier(Modifier::BOLD))]))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            f.render_widget(block, rect);
            let rows = vec![
                Row::new(vec!["/", "Enter Input Mode"]),
                Row::new(vec!["Enter", "Submit Input"]),
                Row::new(vec!["ESC", "Exit Input Mode"]),
                Row::new(vec!["c", "Connect to Server"]),
                Row::new(vec!["d", "Disconnect"]),
                Row::new(vec!["q", "Quit"]),
                Row::new(vec!["?", "Open/Close Help"]),
            ];
            let table = Table::new(rows)
                .header(
                    Row::new(vec!["Key", "Action"])
                        .bottom_margin(1)
                        .style(Style::default().add_modifier(Modifier::BOLD)),
                )
                .widths(&[Constraint::Percentage(10), Constraint::Percentage(90)])
                .column_spacing(10);
            f.render_widget(table, area.inner(&Margin { vertical: 8, horizontal: 24 }));
        };

        f.render_widget(
            Block::default()
                .title(
                    ratatui::widgets::block::Title::from(format!(
                        "{:?}",
                        &self.last_events.iter().map(|k| key_event_to_string(k)).collect::<Vec<_>>()
                    ))
                    .alignment(Alignment::Right),
                )
                .title_style(Style::default().add_modifier(Modifier::BOLD)),
            Rect { x: area.x + 1, y: area.height.saturating_sub(1), width: area.width.saturating_sub(2), height: 1 },
        );

        Ok(())
    }
}
