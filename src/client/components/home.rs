use async_trait::async_trait;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures::{select, FutureExt, Sink, SinkExt};
use log::error;
use once_cell::sync::Lazy;
use ratatui::{prelude::*, widgets::*};
use std::{collections::HashMap, net::SocketAddr, pin::Pin, sync::Mutex, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::mpsc,
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
};

use super::{Component, Frame};

/// Shorthand for the transmit half of the message channel
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receiving half of the message channel
pub type Rx<T> = mpsc::UnboundedReceiver<T>;
/// Shorthand for a pinned boxed stream
pub type BoxedStream<Item> = Pin<Box<dyn Stream<Item = Item> + Send>>;
/// Shorthand for a BoxedStream type we will use
pub type ClientTcpStream = BoxedStream<Result<String, std::io::Error>>;

#[derive(PartialEq)]
pub enum ConnectionStatus {
    CONNECTED,
    DISCONNECTED,
}

/// Client connection status
pub struct ClientStatus {
    status: ConnectionStatus,
}

impl ClientStatus {
    pub fn new() -> Self {
        let status = ConnectionStatus::DISCONNECTED;
        Self { status }
    }
}

/// Task cancellation token
pub struct CancelClient {
    token: CancellationToken,
}

impl CancelClient {
    pub fn new() -> Self {
        let token = CancellationToken::new();
        Self { token }
    }
}

/// Messages queues
pub struct Messages {
    outgoing: Vec<String>,
    text: Vec<String>,
}

impl Messages {
    pub fn new() -> Self {
        let outgoing = Vec::new();
        let text = Vec::new();
        Self { outgoing, text }
    }
}

/// Wrapped read half of a TcpStream
pub struct ClientStream {
    rx: ClientTcpStream,
}

impl ClientStream {
    pub fn new(reader: OwnedReadHalf) -> Self {
        let rx = Box::pin(LinesStream::new(BufReader::new(reader).lines()));
        Self { rx }
    }
}

/// Wrapped write half of a TcpStream
pub struct ClientSink {
    tx: FramedWrite<OwnedWriteHalf, LinesCodec>,
}

impl ClientSink {
    pub fn new(writer: OwnedWriteHalf) -> Self {
        let tx = FramedWrite::new(writer, LinesCodec::new());
        Self { tx }
    }
}

/// Lazy Mutex wrapped global client connection status
static CLIENT_STATUS: Lazy<Mutex<ClientStatus>> = Lazy::new(|| {
    let cs = ClientStatus::new();
    Mutex::new(cs)
});

/// Lazy Mutex wrapped global cancellation token
static CANCEL_TOKEN: Lazy<Mutex<CancelClient>> = Lazy::new(|| {
    let c = CancelClient::new();
    Mutex::new(c)
});

/// Lazy Mutex wrapped global message buffers
static MESSAGES: Lazy<Mutex<Messages>> = Lazy::new(|| {
    let m = Messages::new();
    Mutex::new(m)
});

fn split_tcp_stream(stream: TcpStream) -> Result<(ClientStream, ClientSink)> {
    let (reader, writer) = stream.into_split();
    Ok((ClientStream::new(reader), ClientSink::new(writer)))
}

async fn get_user_input(mut input: Input) -> Option<String> {
    let message = input.value().to_string();
    input.reset();
    if message.is_empty() {
        None
    } else {
        Some(message)
    }
}

async fn add_message(s: String) {
    MESSAGES.lock().unwrap().text.insert(0, s);
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
    pub async fn new() -> Home {
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

    async fn get_server_address(&mut self) -> Option<SocketAddr> {
        let user_input = self.input.value().to_string();
        if user_input.is_empty() {
            add_message("Please enter an address string, e.g. 127.0.0.1:8080".to_string()).await;
            return None;
        }
        let address: SocketAddr = match user_input.parse() {
            Ok(addr) => addr,
            _ => {
                add_message("Incorrect server address, try again.".to_string()).await;
                self.input.reset();
                return None;
            },
        };
        self.input.reset();
        Some(address)
    }

    async fn connect_client(&mut self, address: SocketAddr) {
        if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::DISCONNECTED {
            let stream = TcpStream::connect(address).await;
            if stream.is_ok() {
                CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::CONNECTED;
                add_message(format!("Connected to server {}.", address.to_string())).await;
                // let transport = Framed::new(stream.unwrap(), LinesCodec::new());
                let (reader, writer) = split_tcp_stream(stream.unwrap()).unwrap();
                self.net_event_select_loop(reader, writer /* transport */).await;
            } else {
                add_message(format!("Could not connect to server {}.", address.to_string())).await;
            }
        } else {
            add_message("Already connected to server.".to_string()).await;
        }
    }

    async fn disconnect_client(&mut self) {
        if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
            CANCEL_TOKEN.lock().unwrap().token.cancel();
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            add_message("Disconnected from server.".to_string()).await;
            CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::DISCONNECTED;
        } else {
            add_message("Not connected to a server.".to_string()).await;
        }
        CANCEL_TOKEN.lock().unwrap().token = CancellationToken::new();
    }

    pub async fn net_event_select_loop(
        &mut self,
        reader: ClientStream,
        writer: ClientSink, /* mut transport: Framed<TcpStream, LinesCodec> */
    ) {
        let input = self.input.clone();
        let mut stream = reader;
        let mut sink = writer;
        let cancel_token = CANCEL_TOKEN.lock().unwrap().token.clone();
        tokio::spawn(async move {
            loop {
                if !MESSAGES.lock().unwrap().outgoing.is_empty() {
                    let outgoing: Vec<String> =
                        MESSAGES.lock().unwrap().outgoing.clone().iter().map(|l| String::from(l.clone())).collect();
                    for message in outgoing {
                        add_message(format!("{}{}", "You: ".to_owned(), message.clone())).await;
                        // let _ = transport.send(message.clone()).await;
                        let _ = sink.tx.send(message.clone()).await;
                        MESSAGES.lock().unwrap().outgoing.clear();
                    }
                }
                select! {
                    _ = cancel_token.cancelled().fuse() => {
                        let mut writer = sink.tx.into_inner();
                        let _ = writer.shutdown();
                        break;
                    },
                    // message = transport.next().fuse() => match message {
                    message = stream.rx.next().fuse() => match message {
                        Some(Ok(message)) => add_message(message.clone()).await,
                        None => {
                            add_message("The Lair has CLOSED.".to_string()).await;
                            CANCEL_TOKEN.lock().unwrap().token.cancel();
                            CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::DISCONNECTED;
                            CANCEL_TOKEN.lock().unwrap().token = CancellationToken::new();
                            break;
                        },
                        _ => continue,
                    },
                    message = get_user_input(input.clone()).fuse() => match message {
                        Some(message) => {
                            // let _ = transport.send(message.clone()).await;
                            let _ = sink.tx.send(message.clone()).await;
                            add_message(format!("{}{}", "You: ".to_owned(), message.to_string())).await;
                        },
                        None => continue,
                    }
                }
            }
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

#[async_trait(?Send)]
impl Component for Home {
    fn register_action_handler(&mut self, tx: Tx<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    async fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        self.last_events.push(key.clone());
        let action = match self.mode {
            Mode::Normal | Mode::Processing => match key.code {
                KeyCode::Char('q') => {
                    self.disconnect_client().await;
                    Action::Quit
                },
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.disconnect_client().await;
                    Action::Update
                },
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.disconnect_client().await;
                    Action::Quit
                },
                KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Suspend,
                KeyCode::Char('?') => Action::ToggleShowHelp,
                KeyCode::Char('/') => Action::EnterInsert,
                KeyCode::Char('c') => {
                    let address = self.get_server_address().await;
                    if address.is_some() {
                        self.connect_client(address.unwrap()).await;
                    }
                    Action::Update
                },
                KeyCode::Char('d') => {
                    self.disconnect_client().await;
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
                KeyCode::Esc => Action::EnterNormal,
                KeyCode::Enter => {
                    let message = get_user_input(self.input.clone()).await;
                    if message.is_some() && CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
                        MESSAGES.lock().unwrap().outgoing.insert(0, message.unwrap());
                        self.input.reset();
                    }
                    Action::Update
                },
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    Action::Update
                },
            },
            _ => {
                self.input.handle_event(&crossterm::event::Event::Key(key));
                Action::Tick
            },
        };
        Ok(Some(action))
    }

    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
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
                self.mode = Mode::Insert;
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

        f.render_widget(
            Paragraph::new(text)
                .block(
                    Block::default()
                        .title("The Lair v0.1.0 (c) 2023 Michael Berry")
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
                Row::new(vec!["d", "Disconnect from Server"]),
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
