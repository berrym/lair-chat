use futures::SinkExt;
use std::{collections::HashMap, env, error::Error, io, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use tracing_subscriber::fmt::format::FmtSpan;

/// Shorthand for the transmit half of the message channel.
pub type Tx<T> = mpsc::UnboundedSender<T>;
/// Shorthand for the receive half of the message channel.
pub type Rx<T> = mpsc::UnboundedReceiver<T>;

/// Data that is shared between all peers in the chat server.
///
/// This is the set of `Tx` handles for all connected clients. Whenever a
/// message is received from a client, it is broadcasted to all peers by
/// iterating over the `peers` entries and sending a copy of the message on each
/// `Tx`.
struct SharedState {
    peers: HashMap<SocketAddr, Tx<String>>,
    nicknames: Vec<String>,
}

impl SharedState {
    /// Create a new, empty, instance of `SharedState`.
    fn new() -> Self {
        SharedState { peers: HashMap::new(), nicknames: Vec::new() }
    }

    /// Send a `LineCodec` encoded message to every peer, except
    /// for the sender.
    async fn broadcast(&mut self, sender: SocketAddr, message: &str) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != sender {
                let _ = peer.1.send(message.into());
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
    async fn new(state: Arc<Mutex<SharedState>>, transport: Framed<TcpStream, LinesCodec>) -> io::Result<Peer> {
        let mut state = state.lock().await;

        // Get the client socket address
        let addr = transport.get_ref().peer_addr()?;

        // Create a channel for this peer
        let (tx, rx) = mpsc::unbounded_channel();

        // Add an entry for this `Peer` in the shared state map.
        state.peers.insert(addr, tx);

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

    let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Bind a TCP listener to the socket address.
    //
    // Note that this is the Tokio TcpListener, which is fully async.
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("server running on {}", addr);

    loop {
        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await?;

        // Clone a handle to the `SharedState` for the new connection.
        let state = Arc::clone(&state);

        // Spawn our handler to be run asynchronously.
        tokio::spawn(async move {
            tracing::debug!("accepted connection");
            if let Err(e) = process(state, stream, addr).await {
                tracing::info!("an error occurred; error = {:?}", e);
            }
        });
    }
}

/// Process an individual chat client
async fn process(state: Arc<Mutex<SharedState>>, stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut transport = Framed::new(stream, LinesCodec::new());

    // Send a welcome prompt to the client then askk client to enter their nickname.
    transport.send("Welcome to The Lair!").await?;
    transport.send("Please enter a nickname:").await?;

    // Read from the `LinesCodec` stream to get the nickname.
    let mut nickname = String::new();
    loop {
        match transport.next().await {
            Some(Ok(message)) => {
                let mut state = state.lock().await;
                if state.nicknames.is_empty() {
                    state.nicknames = vec!["You".into(), "you".into(), "me".into(), "Me".into()];
                }
                if state.nicknames.contains(&message.clone()) {
                    transport.send("Nickname already in use, try again:").await?;
                } else {
                    nickname = message;
                    state.nicknames.push(nickname.clone());
                    transport.send(format!("You've logged in to The Lair, happy chatting {}", nickname)).await?;
                    break;
                }
            },
            // We didn't get a message so we return early here.
            _ => {
                drop(nickname);
                tracing::error!("Failed to get nickname from {}. Client disconnected.", addr);
                return Ok(());
            },
        };
    }

    // Register our peer with state which internally sets up some channels.
    let mut peer = Peer::new(state.clone(), transport).await?;

    // A client has connected, let's let everyone know.
    {
        let mut state = state.lock().await;
        let message = format!("{} has joined the chat", nickname);
        tracing::info!("{}", message);
        state.broadcast(addr, &message).await;
    }

    // Process incoming messages until our stream is exhausted by a disconnect.
    loop {
        tokio::select! {
            // A message was received from a peer. Send it to the current user.
            Some(message) = peer.rx.recv() => {
                peer.transport.send(&message).await?;
            }
            result = peer.transport.next() => match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Some(Ok(message)) => {
                    let mut state = state.lock().await;
                    let message = format!("{}: {}", nickname, message);

                    state.broadcast(addr, &message).await;
                }
                // An error occurred.
                Some(Err(e)) => {
                    tracing::error!(
                        "an error occurred while processing messages for {}; error = {:?}",
                        nickname,
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

        let index = state.nicknames.iter().position(|s| *s == nickname).unwrap();
        state.nicknames.remove(index);

        let message = format!("{} has left the chat", nickname);
        tracing::info!("{}", message);
        state.broadcast(addr, &message).await;
    }

    Ok(())
}
