use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use color_eyre::eyre::Result;
use futures::{select, FutureExt, SinkExt};
use md5;
use once_cell::sync::Lazy;
use std::{net::SocketAddr, pin::Pin, sync::Mutex};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    time::{sleep, Duration},
};
use tokio_stream::{wrappers::LinesStream, Stream, StreamExt};
use tokio_util::{
    codec::{FramedWrite, LinesCodec},
    sync::CancellationToken,
};
use tui_input::Input;
use x25519_dalek::{EphemeralSecret, PublicKey};

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

/// Lazy Mutex wrapped global client connection status
pub static CLIENT_STATUS: Lazy<Mutex<ClientStatus>> = Lazy::new(|| {
    let m = ClientStatus::new();
    Mutex::new(m)
});

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

/// Lazy Mutex wrapped global cancellation token
pub static CANCEL_TOKEN: Lazy<Mutex<CancelClient>> = Lazy::new(|| {
    let m = CancelClient::new();
    Mutex::new(m)
});

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

/// Lazy Mutex wrapped global message buffers
pub static MESSAGES: Lazy<Mutex<Messages>> = Lazy::new(|| {
    let m = Messages::new();
    Mutex::new(m)
});

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
        add_text_message("Connected to server.".to_owned());
        add_text_message("".to_owned());
        tokio::task::spawn_blocking(move || {
            CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::CONNECTED;
            let (reader, writer) = split_tcp_stream(stream.unwrap()).unwrap();
            let _ = client_io_select_loop(input, reader, writer);
        });
    } else {
        add_text_message("Failed to connect to server.".to_owned());
        add_text_message("".to_owned());
    }
}

pub fn client_io_select_loop(
    input: Input,
    reader: ClientStream,
    writer: ClientSink,
) -> Result<(), Box<dyn std::error::Error>> {
    // create private/public keys
    let client_secret_key = EphemeralSecret::random();
    let client_public_key = PublicKey::from(&client_secret_key);
    // create a sink and stream for transport
    let mut stream = reader;
    let mut sink = writer;
    let cancel_token = CANCEL_TOKEN.lock().unwrap().token.clone();
    tokio::spawn(async move {
        // start handshake by sending public key to server
        _ = sink
            .tx
            .send(BASE64_STANDARD.encode(client_public_key))
            .await;
        // recieve server public key
        let server_public_key_string = match stream.rx.next().await {
            Some(key_string) => key_string,
            None => {
                add_text_message("Failed to get server public key!.".to_owned());
                return Ok(());
            }
        };
        // keep converting until key is a 32 byte u8 array
        let server_public_key_vec = match server_public_key_string {
            Ok(key_vec) => BASE64_STANDARD.decode(key_vec).unwrap(),
            _ => {
                add_text_message("Failed to convert server public key to byte vec!".to_owned());
                return Err("failed to convert server public key to byte vec");
            }
        };
        let server_public_key_slice: &[u8] = match server_public_key_vec.as_slice().try_into() {
            Ok(key_slice) => key_slice,
            _ => {
                add_text_message(
                    "Failed to convert server public key byte vec to byte slice!".to_owned(),
                );
                return Err("failed to convert server public key byte vec to byte slice");
            }
        };
        let server_public_key_array: [u8; 32] = match server_public_key_slice.try_into() {
            Ok(key_array) => key_array,
            _ => {
                add_text_message(
                    "Failed to convert public key byte slice to byte array!".to_owned(),
                );
                return Err("failed to convert public key slice to array");
            }
        };
        // create shared keys
        let shared_secret =
            client_secret_key.diffie_hellman(&PublicKey::from(server_public_key_array));
        let shared_aes256_key =
            format!("{:x}", md5::compute(BASE64_STANDARD.encode(shared_secret)));
        // main client loop
        loop {
            // process any messages
            if !MESSAGES.lock().unwrap().outgoing.is_empty() {
                let outgoing: Vec<String> = MESSAGES
                    .lock()
                    .unwrap()
                    .outgoing
                    .clone()
                    .iter()
                    .map(|l| encrypt(shared_aes256_key.clone(), String::from(l.clone())))
                    .collect();
                for message in outgoing {
                    add_text_message(format!(
                        "{}{}",
                        "You: ".to_owned(),
                        decrypt(shared_aes256_key.clone(), message.clone())
                    ));
                    let _ = sink.tx.send(message.clone()).await;
                    MESSAGES.lock().unwrap().outgoing.clear();
                }
            }
            // select on futures
            select! {
                _ = cancel_token.cancelled().fuse() => {
                    let mut writer = sink.tx.into_inner();
                    let _ = writer.shutdown().await;
                    return Ok(());
                },
                message = stream.rx.next().fuse() => match message {
                    Some(Ok(message)) => add_text_message(decrypt(shared_aes256_key.clone(), message.clone())),
                    None => {
                        add_text_message("The Lair has CLOSED.".to_string());
                        CANCEL_TOKEN.lock().unwrap().token.cancel();
                        CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::DISCONNECTED;
                        CANCEL_TOKEN.lock().unwrap().token = CancellationToken::new();
                        return Ok(());
                    },
                    _ => continue,
                },
                message = get_user_input(input.clone()).fuse() => match message {
                    Some(message) => {
                        let _ = sink.tx.send(encrypt(shared_aes256_key.clone(), message.clone())).await;
                        add_text_message(format!("{}{}", "You: ".to_owned(), message.to_owned()));
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
    Ok(())
}

pub async fn disconnect_client() {
    if CLIENT_STATUS.lock().unwrap().status == ConnectionStatus::CONNECTED {
        CANCEL_TOKEN.lock().unwrap().token.cancel();
        sleep(Duration::from_millis(250)).await;
        add_text_message("Disconnected from server.".to_string());
        CLIENT_STATUS.lock().unwrap().status = ConnectionStatus::DISCONNECTED;
    } else {
        add_text_message("Not connected to a server.".to_string());
    }
    CANCEL_TOKEN.lock().unwrap().token = CancellationToken::new();
}

// encrypt strings with Aes256Gcm
fn encrypt(key_str: String, plaintext: String) -> String {
    let key = Key::<Aes256Gcm>::from_slice(key_str.as_bytes());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let cipher = Aes256Gcm::new(key);
    let ciphered_data = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .expect("failed to encrypt");
    // combining nonce and encrypted data together for storage purpose
    let mut encrypted_data: Vec<u8> = nonce.to_vec();
    encrypted_data.extend_from_slice(&ciphered_data);
    BASE64_STANDARD.encode(encrypted_data)
}

// decrypt strings
fn decrypt(key_str: String, encrypted_data: String) -> String {
    let encrypted_data = BASE64_STANDARD.decode(encrypted_data).unwrap();
    let key = Key::<Aes256Gcm>::from_slice(key_str.as_bytes());
    let (nonce_arr, ciphered_data) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_arr);
    let cipher = Aes256Gcm::new(key);
    let plaintext = cipher
        .decrypt(nonce, ciphered_data)
        .expect("failed to decrypt data");
    String::from_utf8(plaintext).expect("failed to convert vector of bytes to string")
}
