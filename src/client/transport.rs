use color_eyre::eyre::Result;
use futures::{select, FutureExt, Sink, SinkExt};
use log::error;
use once_cell::sync::Lazy;
use std::{net::SocketAddr, pin::Pin, sync::Mutex};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::mpsc,
    time::{sleep, Duration},
};
use tokio_stream::{wrappers::LinesStream, Stream, StreamExt};
use tokio_util::{
    codec::{FramedWrite, LinesCodec},
    sync::CancellationToken,
};
use tui_input::Input;

use crate::components::home::get_user_input;

/// Shorthand for a pinned boxed stream
pub type BoxedStream<Item> = Pin<Box<dyn Stream<Item = Item> + Send>>;
/// Shorthand for a lines framed BoxedStream type we will use
pub type ClientTcpStream = BoxedStream<Result<String, std::io::Error>>;

#[derive(PartialEq)]
pub enum ConnectionStatus {
    CONNECTED,
    DISCONNECTED,
}

/// Client connection status
pub struct ClientStatus {
    pub status: ConnectionStatus,
}

impl ClientStatus {
    pub fn new() -> Self {
        let status = ConnectionStatus::DISCONNECTED;
        Self { status }
    }
}

/// Task cancellation token
pub struct CancelClient {
    pub token: CancellationToken,
}

impl CancelClient {
    pub fn new() -> Self {
        let token = CancellationToken::new();
        Self { token }
    }
}

/// Wrapped read half of a TcpStream
pub struct ClientStream {
    pub rx: ClientTcpStream,
}

impl ClientStream {
    pub fn new(reader: OwnedReadHalf) -> Self {
        let rx = Box::pin(LinesStream::new(BufReader::new(reader).lines()));
        Self { rx }
    }
}

/// Wrapped write half of a TcpStream
pub struct ClientSink {
    pub tx: FramedWrite<OwnedWriteHalf, LinesCodec>,
}

impl ClientSink {
    pub fn new(writer: OwnedWriteHalf) -> Self {
        let tx = FramedWrite::new(writer, LinesCodec::new());
        Self { tx }
    }
}

/// Messages buffers
pub struct Messages {
    pub outgoing: Vec<String>,
    pub text: Vec<String>,
}

impl Messages {
    pub fn new() -> Self {
        let outgoing = Vec::new();
        let text = Vec::new();
        Self { outgoing, text }
    }
}

/// Add a message to displayed in the main window
pub fn add_text_message(s: String) {
    MESSAGES.lock().unwrap().text.push(s);
}

/// Add a message to the outgoing buffer
pub fn add_outgoing_message(s: String) {
    MESSAGES.lock().unwrap().outgoing.insert(0, s);
}

pub fn split_tcp_stream(stream: TcpStream) -> Result<(ClientStream, ClientSink)> {
    let (reader, writer) = stream.into_split();
    Ok((ClientStream::new(reader), ClientSink::new(writer)))
}

pub async fn connect_client(input: Input, address: SocketAddr) {
    add_text_message(format!("Connecting to {}", address.clone().to_string()).to_string());
    let stream = TcpStream::connect(address).await;
    if stream.is_ok() {
        tokio::task::spawn_blocking(move || {
            CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::CONNECTED;
            let (reader, writer) = split_tcp_stream(stream.unwrap()).unwrap();
            client_io_select_loop(input, reader, writer);
        });
    }
}

pub fn client_io_select_loop(input: Input, reader: ClientStream, writer: ClientSink) {
    if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
        add_text_message("Connected to server.".to_string());
    } else {
        add_text_message("Failed to connect to server.".to_string());
        return;
    }
    let mut stream = reader;
    let mut sink = writer;
    let cancel_token = CANCEL_TOKEN.lock().unwrap().token.clone();
    tokio::spawn(async move {
        loop {
            if !MESSAGES.lock().unwrap().outgoing.is_empty() {
                let outgoing: Vec<String> =
                    MESSAGES.lock().unwrap().outgoing.clone().iter().map(|l| String::from(l.clone())).collect();
                for message in outgoing {
                    add_text_message(format!("{}{}", "You: ".to_owned(), message.clone()));
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
                message = stream.rx.next().fuse() => match message {
                    Some(Ok(message)) => add_text_message(message.clone()),
                    None => {
                        add_text_message("The Lair has CLOSED.".to_string());
                        CANCEL_TOKEN.lock().unwrap().token.cancel();
                        CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::DISCONNECTED;
                        CANCEL_TOKEN.lock().unwrap().token = CancellationToken::new();
                        break;
                    },
                    _ => continue,
                },
                message = get_user_input(input.clone()).fuse() => match message {
                    Some(message) => {
                        let _ = sink.tx.send(message.clone()).await;
                        add_text_message(format!("{}{}", "You: ".to_owned(), message.to_string()));
                        sleep(Duration::from_millis(250)).await;
                    },
                    None => {
                        sleep(Duration::from_millis(250)).await;
                        continue;
                    },
                }
            }
        }
    });
}

pub async fn disconnect_client() {
    if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
        CANCEL_TOKEN.lock().unwrap().token.cancel();
        sleep(Duration::from_millis(500)).await;
        add_text_message("Disconnected from server.".to_string());
        CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::DISCONNECTED;
    } else {
        add_text_message("Not connected to a server.".to_string());
    }
    CANCEL_TOKEN.lock().unwrap().token = CancellationToken::new();
}

/// Lazy Mutex wrapped global client connection status
pub static CLIENT_STATUS: Lazy<Mutex<ClientStatus>> = Lazy::new(|| {
    let m = ClientStatus::new();
    Mutex::new(m)
});

/// Lazy Mutex wrapped global cancellation token
pub static CANCEL_TOKEN: Lazy<Mutex<CancelClient>> = Lazy::new(|| {
    let m = CancelClient::new();
    Mutex::new(m)
});

/// Lazy Mutex wrapped global message buffers
pub static MESSAGES: Lazy<Mutex<Messages>> = Lazy::new(|| {
    let m = Messages::new();
    Mutex::new(m)
});
