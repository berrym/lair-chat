use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use futures::SinkExt;
use md5;
use std::{collections::HashMap, env, error::Error, io, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use tracing_subscriber::fmt::format::FmtSpan;
use x25519_dalek::{EphemeralSecret, PublicKey};

mod auth;
use auth::{
    AuthError, AuthRequest, AuthService, MemorySessionStorage,
    MemoryUserStorage, Session, User, UserStorage,
};

/// Shorthand for the transmit half of the message channel.
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receive half of the message channel.
pub type Rx<T> = mpsc::UnboundedReceiver<T>;
pub type WriteData = (String, Tx<String>);

/// Data that is shared between all peers in the chat server.
///
/// This is the set of `Tx` handles for all connected clients. Whenever a
/// message is received from a client, it is broadcasted to all peers by
/// iterating over the `peers` entries and sending a copy of the message on each
/// `Tx`.
struct SharedState {
    peers: HashMap<SocketAddr, WriteData>,
    auth_service: Arc<AuthService>,
}

impl SharedState {
    /// Create a new, empty, instance of `SharedState`.
    fn new() -> Self {
        let user_storage = Arc::new(MemoryUserStorage::new());
        let session_storage = Arc::new(MemorySessionStorage::new());
        let auth_service = Arc::new(AuthService::new(user_storage.clone(), session_storage, None));
        
        // Add default test users for easier testing
        tokio::spawn(async move {
            if let Ok(user1) = User::new("lusus".to_string(), "c2nt3ach") {
                let _ = user_storage.create_user(user1).await;
            }
            if let Ok(user2) = User::new("alice".to_string(), "password123") {
                let _ = user_storage.create_user(user2).await;
            }
            if let Ok(user3) = User::new("bob".to_string(), "password456") {
                let _ = user_storage.create_user(user3).await;
            }
            tracing::info!("Default test users created");
        });
        
        SharedState {
            peers: HashMap::new(),
            auth_service,
        }
    }

    /// Send a `LineCodec` encoded message to every peer, except
    /// for the sender.
    async fn broadcast(&mut self, sender: SocketAddr, message: &str) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != sender {
                let _ = peer
                    .1
                     .1
                    .send(encrypt(peer.1 .0.to_string().clone(), message.into()));
            }
        }
    }
}

/// The state for each connected client.
struct Peer {
    /// The TCP socket wrapped with the `Lines` codec, defined below.
    ///
    /// This handles sending and receiving data on the socket. When using
    /// `Lines`, we can work at the line level instead of having to manage the
    /// raw byte operations.
    transport: Framed<TcpStream, LinesCodec>,

    /// Receive half of the message channel.
    ///
    /// This is used to receive messages from peers. When a message is received
    /// off of this `Rx`, it will be written to the socket.
    rx: Rx<String>,
}

impl Peer {
    /// Create a new instance of `Peer`.
    async fn new(
        state: Arc<Mutex<SharedState>>,
        transport: Framed<TcpStream, LinesCodec>,
        shared_aes_key: String,
    ) -> io::Result<Peer> {
        let mut state = state.lock().await;

        // Get the client socket address
        let addr = transport.get_ref().peer_addr()?;

        // Create a channel for this peer
        let (tx, rx) = mpsc::unbounded_channel();

        // Create a shared key/transport writer tuple
        let write_data = (shared_aes_key.clone(), tx);

        // Add an entry for this `Peer` in the shared state map.
        state.peers.insert(addr, write_data);

        Ok(Peer { transport, rx })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Configure a `tracing` subscriber that logs traces emitted by the chat
    // server.
    tracing_subscriber::fmt()
        // Filter what traces are displayed based on the RUST_LOG environment
        // variable.
        //
        // Traces emitted by the example code will always be displayed. You
        // can set `RUST_LOG=tokio=trace` to enable additional traces emitted by
        // Tokio itself.
        // .with_env_filter(EnvFilter::from_default_env().add_directive("chat=info".parse()?))
        // Log events when `tracing` spans are created, entered, exited, or
        // closed. When Tokio's internal tracing support is enabled (as
        // described above), this can be used to track the lifecycle of spawned
        // tasks on the Tokio runtime.
        .with_span_events(FmtSpan::FULL)
        // Set this subscriber as the default, to collect all traces emitted by
        // the program.
        .init();

    // Create the shared state. This is how all the peers communicate.
    //
    // The server task will hold a handle to this. For every new client, the
    // `state` handle is cloned and passed into the task that processes the
    // client connection.
    let state = Arc::new(Mutex::new(SharedState::new()));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Bind a TCP listener to the socket address.
    //
    // Note that this is the Tokio TcpListener, which is fully async.
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("server running on {}", addr);

    loop {
        let server_secret_key = EphemeralSecret::random();
        let server_public_key = PublicKey::from(&server_secret_key);

        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await?;

        // Clone a handle to the `SharedState` for the new connection.
        let state = Arc::clone(&state);

        // Spawn our handler to be run asynchronously.
        tokio::spawn(async move {
            tracing::debug!("accepted connection");
            if let Err(e) = process(state, stream, addr, server_public_key, server_secret_key).await
            {
                tracing::info!("an error occurred; error = {:?}", e);
            }
        });
    }
}

/// Process an individual chat client
async fn process(
    state: Arc<Mutex<SharedState>>,
    stream: TcpStream,
    addr: SocketAddr,
    server_public_key: PublicKey,
    server_secret_key: EphemeralSecret,
) -> Result<(), Box<dyn Error>> {
    // start the handshake by sending public key to peer
    let mut transport = Framed::new(stream, LinesCodec::new());
    transport
        .send(BASE64_STANDARD.encode(server_public_key))
        .await?;
    // recieve peer's public key
    let peer_public_key_string = match transport.next().await {
        Some(key_string) => {
            println!("Got public key from: {}", addr.to_string());
            key_string
        }
        None => {
            println!("Failed to get public key from peer!");
            return Ok(());
        }
    };
    // keep converting until key is a 32 byte u8 array
    let peer_public_key_vec = match peer_public_key_string {
        Ok(key_vec) => BASE64_STANDARD.decode(key_vec).unwrap(),
        _ => {
            println!("Failed to convert peer public key to byte vec!");
            return Ok(());
        }
    };
    let peer_public_key_slice: &[u8] = match peer_public_key_vec.as_slice().try_into() {
        Ok(key_slice) => key_slice,
        _ => {
            println!("Failed to convert peer public key byte vec to slice!");
            return Ok(());
        }
    };
    let peer_public_key_array: [u8; 32] = match peer_public_key_slice.try_into() {
        Ok(key_array) => key_array,
        _ => {
            println!("Failed to convert public key slice to byte array!");
            return Ok(());
        }
    };
    // create shared keys
    let shared_secret = server_secret_key.diffie_hellman(&PublicKey::from(peer_public_key_array));
    let shared_aes256_key = format!("{:x}", md5::compute(BASE64_STANDARD.encode(shared_secret)));

    // Send a welcome prompt to the client
    transport
        .send(encrypt(
            shared_aes256_key.clone(),
            "Welcome to The Lair! Please login or register.".to_string(),
        ))
        .await?;

    // Handle authentication
    let mut user = None;
    let mut session = None;

    while user.is_none() {
        match transport.next().await {
            Some(Ok(message)) => {
                let auth_request: AuthRequest = serde_json::from_str(
                    &decrypt(shared_aes256_key.clone(), message)
                ).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid auth request"))?;

                let state = state.lock().await;
                let result = if auth_request.is_registration {
                    state.auth_service.register(
                        auth_request.username.clone(),
                        &auth_request.password,
                    ).await
                } else {
                    match state.auth_service.login(auth_request).await {
                        Ok(response) => {
                            session = Some(response.session);
                            Ok(response.user)
                        }
                        Err(e) => Err(e),
                    }
                };

                match result {
                    Ok(authenticated_user) => {
                        user = Some(authenticated_user.clone());
                        transport
                            .send(encrypt(
                                shared_aes256_key.clone(),
                                format!("Welcome back, {}!", authenticated_user.username),
                            ))
                            .await?;
                        break;
                    }
                    Err(e) => {
                        transport
                            .send(encrypt(
                                shared_aes256_key.clone(),
                                format!("Authentication failed: {}", e),
                            ))
                            .await?;
                    }
                }
            }
            _ => {
                tracing::error!("Client disconnected during authentication: {}", addr);
                return Ok(());
            }
        }
    }

    let user = user.unwrap();
    let session = session.unwrap();

    // Register our peer with state which internally sets up some channels.
    let mut peer = Peer::new(state.clone(), transport, shared_aes256_key.clone()).await?;

    // A client has connected, let's let everyone know.
    {
        let mut state = state.lock().await;
        let message = format!("{} has joined the chat", user.username);
        tracing::info!("{}", message);
        state.broadcast(addr, &message).await;
    }

    // Process incoming messages until our stream is exhausted by a disconnect.
    loop {
        tokio::select! {
            // A message was received from a peer. Send it to the current user.
            Some(message) = peer.rx.recv() => {
                peer.transport.send(message).await?;
            }
            result = peer.transport.next() => match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Some(Ok(message)) => {
                    let message = decrypt(shared_aes256_key.clone(), message);
                    let mut state = state.lock().await;
                    let message = format!("{}: {}", user.username, message);

                    state.broadcast(addr, &message).await;
                }
                // An error occurred.
                Some(Err(e)) => {
                    tracing::error!(
                        "an error occurred while processing messages for {}; error = {:?}",
                        user.username,
                        e
                    );
                }
                // The stream has been exhausted.
                None => break,
            },
        }
    }

    // If this section is reached it means that the client was disconnected!
    // Let's let everyone still connected know about it.
    {
        let mut state = state.lock().await;
        state.peers.remove(&addr);

        // Cleanup user session
        if let Err(e) = state.auth_service.logout(&session.token).await {
            tracing::error!("Failed to cleanup session: {}", e);
        }

        let message = format!("{} has left the chat", user.username);
        tracing::info!("{}", message);
        state.broadcast(addr, &message).await;
    }

    Ok(())
}

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

fn decrypt(key_str: String, encrypted_data: String) -> String {
    let encrypted_data = BASE64_STANDARD.decode(encrypted_data).unwrap(); // hex::decode(encrypted_data).expect("failed to decode hex string into vec");
    let key = Key::<Aes256Gcm>::from_slice(key_str.as_bytes());
    let (nonce_arr, ciphered_data) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_arr);
    let cipher = Aes256Gcm::new(key);
    let plaintext = cipher
        .decrypt(nonce, ciphered_data)
        .expect("failed to decrypt data");
    String::from_utf8(plaintext).expect("failed to convert vector of bytes to string")
}
