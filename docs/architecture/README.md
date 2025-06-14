# Lair Chat Architecture Documentation

This document provides a comprehensive overview of the Lair Chat architecture, covering system design, component interactions, data flow, and key architectural decisions.

## Table of Contents

1. [System Overview](#system-overview)
2. [Architecture Principles](#architecture-principles)
3. [Component Architecture](#component-architecture)
4. [Data Flow Diagrams](#data-flow-diagrams)
5. [Security Architecture](#security-architecture)
6. [Transport Layer](#transport-layer)
7. [Storage Architecture](#storage-architecture)
8. [Scalability Considerations](#scalability-considerations)
9. [Performance Architecture](#performance-architecture)
10. [Deployment Architecture](#deployment-architecture)

## System Overview

Lair Chat is a distributed, real-time chat application built with Rust, featuring a client-server architecture with end-to-end encryption, direct messaging, and room-based communication.

### High-Level Architecture

```mermaid
graph TB
    subgraph "Client Applications"
        C1[Client 1]
        C2[Client 2]
        C3[Client N]
    end
    
    subgraph "Common Layer"
        COMMON[Protocol • Crypto • Transport • Errors]
    end
    
    subgraph "Server Tier"
        LB[Load Balancer]
        S1[Server Instance 1]
        S2[Server Instance 2]
        S3[Server Instance N]
    end
    
    subgraph "Data Tier"
        DB[(Database)]
        CACHE[(Redis Cache)]
        FS[(File Storage)]
    end
    
    C1 <--> COMMON
    C2 <--> COMMON
    C3 <--> COMMON
    
    COMMON <--> LB
    
    LB <--> S1
    LB <--> S2
    LB <--> S3
    
    S1 <--> DB
    S2 <--> DB
    S3 <--> DB
    
    S1 <--> CACHE
    S2 <--> CACHE
    S3 <--> CACHE
    
    S1 <--> FS
    S2 <--> FS
    S3 <--> FS
```

### Modular Project Structure

```
src/
├── bin/                    # Binary entry points
│   ├── client.rs          # Client application entry
│   └── server.rs          # Server application entry
├── common/                 # Shared functionality
│   ├── protocol/          # Message types & protocols  
│   ├── crypto/            # Encryption utilities
│   ├── transport/         # Network abstractions
│   └── errors/            # Common error types
├── client/                 # Client-specific code
│   ├── ui/components/     # UI components & TUI
│   ├── chat/              # Chat functionality
│   ├── auth/              # Client authentication
│   └── app/               # Application logic
└── server/                 # Server-specific code
    ├── app/               # Server application logic
    ├── chat/              # Message & room handling
    ├── auth/              # Server authentication
    └── network/           # Connection management
```

### Technology Stack

| Layer | Technologies |
|-------|-------------|
| **Client** | Rust, Ratatui, Tokio, Crossterm |
| **Server** | Rust, Tokio, Serde, Clap |
| **Transport** | TCP, WebSocket (future), HTTP/2 (future) |
| **Encryption** | AES-256-GCM, X25519, Argon2 |
| **Serialization** | JSON, MessagePack (future) |
| **Storage** | SQLite (embedded), PostgreSQL (production) |
| **Caching** | In-memory, Redis (distributed) |

## Architecture Principles

### Core Principles

1. **Security First**
   - End-to-end encryption by default
   - Zero-trust architecture
   - Secure by design, not by addition

2. **Performance**
   - Async/await throughout
   - Zero-copy where possible
   - Efficient serialization

3. **Reliability**
   - Graceful degradation
   - Automatic reconnection
   - Error recovery

4. **Scalability**
   - Horizontal scaling capability
   - Stateless server design
   - Efficient resource utilization

5. **Maintainability**
   - Clean separation of concerns
   - Dependency injection
   - Comprehensive testing

### Design Patterns

- **Observer Pattern**: Event-driven UI updates
- **Strategy Pattern**: Pluggable transport and encryption
- **Factory Pattern**: Component creation and configuration
- **Command Pattern**: Message handling and routing
- **Singleton Pattern**: Global configuration and state

## Component Architecture

### Client Architecture

The client follows a layered architecture with clear separation of concerns:

```mermaid
graph TD
    subgraph "Client Application (src/client/)"
        UI[UI Layer - components/]
        CHAT[Chat Layer - chat/]
        AUTH[Auth Layer - auth/]
        APP[App Layer - app/]
        NET[Network Layer - network/]
    end
    
    subgraph "Common Layer (src/common/)"
        PROTO[Protocol]
        CRYPTO[Crypto]
        TRANS[Transport]
        ERR[Errors]
    end
    
    UI --> CHAT
    UI --> AUTH
    UI --> APP
    CHAT --> NET
    AUTH --> NET
    APP --> NET
    
    NET --> PROTO
    NET --> CRYPTO
    NET --> TRANS
    NET --> ERR
    
    PROTO --> Server
    CRYPTO --> Server
    TRANS --> Server
```
    subgraph "Presentation Layer"
        TUI[Terminal UI]
        COMP[UI Components]
        EVENTS[Event Handler]
    end
    
    subgraph "Application Layer"
        APP[Application Core]
        CONFIG[Configuration]
        HISTORY[Command History]
    end
    
    subgraph "Domain Layer"
        CHAT[Chat Manager]
        DM[DM Manager]
        ROOM[Room Manager]
        USER[User Manager]
    end
    
    subgraph "Infrastructure Layer"
        CONN[Connection Manager]
        TRANS[Transport Layer]
        CRYPTO[Encryption Service]
        AUTH[Auth Service]
        STORE[Local Storage]
    end
    
    TUI --> APP
    COMP --> APP
    EVENTS --> APP
    
    APP --> CHAT
    APP --> DM
    APP --> ROOM
    APP --> USER
    
    CHAT --> CONN
    DM --> CONN
    ROOM --> CONN
    USER --> CONN
    
    CONN --> TRANS
    CONN --> CRYPTO
    CONN --> AUTH
    CONN --> STORE
```

#### Client Components

1. **Terminal UI (TUI)**
   - Renders the user interface using Ratatui
   - Handles keyboard and mouse input
   - Manages screen layout and widgets

2. **Application Core**
   - Coordinates between UI and business logic
   - Manages application state
   - Handles configuration

3. **Chat Manager**
   - Manages chat rooms and conversations
   - Handles message history
   - Coordinates with UI for display

4. **Connection Manager**
   - Manages server connections
   - Handles reconnection logic
   - Coordinates transport and encryption

5. **Transport Layer**
   - Abstracts network communication
   - Supports multiple protocols
   - Handles connection lifecycle

### Server Architecture

The server follows a modular architecture with clear separation of concerns:

```mermaid
graph TD
    subgraph "Server Application (src/server/)"
        APP_LAYER[App Layer - app/]
        CHAT_LAYER[Chat Layer - chat/]
        AUTH_LAYER[Auth Layer - auth/]
        NET_LAYER[Network Layer - network/]
    end
    
    subgraph "Common Layer (src/common/)"
        PROTO[Protocol]
        CRYPTO[Crypto]
        TRANS[Transport]
        ERR[Errors]
    end
    
    subgraph "Infrastructure"
        DB[(Database)]
        CACHE[(Cache)]
        STORAGE[(File Storage)]
    end
    
    APP_LAYER --> CHAT_LAYER
    APP_LAYER --> AUTH_LAYER
    APP_LAYER --> NET_LAYER
    
    CHAT_LAYER --> PROTO
    AUTH_LAYER --> PROTO
    NET_LAYER --> TRANS
    
    PROTO --> CRYPTO
    TRANS --> CRYPTO
    
    AUTH_LAYER --> DB
    CHAT_LAYER --> DB
    NET_LAYER --> CACHE
    
    CHAT_LAYER --> STORAGE
```

#### Detailed Server Components

```mermaid
graph TD
    subgraph "app/ - Application Logic"
        SERVER[ChatServer]
        STATE[SharedState]
        CONFIG[ServerConfig]
    end
    
    subgraph "chat/ - Message Handling"
        MSG_STORE[MessageStore]
        ROOM_MGR[RoomManager]
        USER_MGR[UserManager]
    end
    
    subgraph "auth/ - Authentication"
        AUTH_SVC[AuthService]
        USER_STORE[UserStorage]
        SESSION_STORE[SessionStorage]
    end
    
    subgraph "network/ - Connection Management"
        CONN_HANDLER[ConnectionHandler]
        SESSION_MGR[SessionManager]
    end
    
    SERVER --> STATE
    SERVER --> CONFIG
    STATE --> ROOM_MGR
    STATE --> USER_MGR
    STATE --> AUTH_SVC
    
    CONN_HANDLER --> SESSION_MGR
    CONN_HANDLER --> AUTH_SVC
    CONN_HANDLER --> MSG_STORE
    
    AUTH_SVC --> USER_STORE
    AUTH_SVC --> SESSION_STORE
```

#### Server Components

1. **ChatServer (src/server/app/server.rs)**
   - Main server application entry point
   - Manages server configuration and lifecycle
   - Coordinates all server components

2. **SharedState (src/server/app/state.rs)**
   - Central state management for the server
   - Manages peer connections and user sessions
   - Handles room assignments and user tracking

3. **RoomManager (src/server/chat/rooms.rs)**
   - Manages chat rooms and participants
   - Handles room creation, deletion, and permissions
   - Coordinates room-based message broadcasting

4. **UserManager (src/server/chat/users.rs)**
   - Tracks connected users and their presence
   - Manages user sessions and activity
   - Handles user movement between rooms

5. **MessageStore (src/server/chat/messages.rs)**
   - Persists and retrieves chat messages
   - Handles message validation and lifecycle
   - Supports direct messages and room messages

6. **AuthService (src/server/auth/service.rs)**
   - Handles user authentication and authorization
   - Manages sessions and tokens
   - Implements rate limiting and security policies

7. **ConnectionHandler (src/server/network/connection_handler.rs)**
   - Manages individual client connections
   - Handles connection lifecycle and cleanup
   - Coordinates authentication and message processing

8. **SessionManager (src/server/network/session_manager.rs)**
   - Tracks active user sessions
   - Manages session timeouts and cleanup
   - Provides session statistics and monitoring

## Data Flow Diagrams

### Message Flow

```mermaid
sequenceDiagram
    participant U1 as User 1
    participant C1 as Client 1
    participant S as Server
    participant C2 as Client 2
    participant U2 as User 2
    
    U1->>C1: Type message
    C1->>C1: Encrypt message
    C1->>S: Send encrypted message
    S->>S: Decrypt & validate
    S->>S: Route to recipients
    S->>S: Encrypt for recipients
    S->>C2: Forward encrypted message
    C2->>C2: Decrypt message
    C2->>U2: Display message
    
    Note over S: Message persistence
    S->>S: Store in database
```

### Authentication Flow

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server
    participant DB as Database
    participant CACHE as Cache
    
    C->>S: Connection request
    S->>C: Challenge + public key
    C->>C: Generate key pair
    C->>S: Public key + credentials
    S->>DB: Validate credentials
    DB->>S: User data
    S->>S: Generate session token
    S->>CACHE: Store session
    S->>C: Session token + encrypted
    C->>C: Store session locally
    
    Note over C,S: Authenticated session established
```

### Direct Message Flow

```mermaid
sequenceDiagram
    participant A as Alice
    participant AC as Alice's Client
    participant S as Server
    participant BC as Bob's Client
    participant B as Bob
    
    A->>AC: Send DM to Bob
    AC->>AC: Encrypt with Bob's key
    AC->>S: Encrypted DM
    S->>S: Validate sender/recipient
    S->>S: Store encrypted DM
    S->>BC: Forward encrypted DM
    BC->>BC: Decrypt with Alice's key
    BC->>B: Display message
    
    alt Bob is offline
        S->>S: Queue message
        Note over S: Message delivered when Bob comes online
    end
```

## Security Architecture

### Encryption Architecture

```mermaid
graph TD
    subgraph "Key Management"
        KG[Key Generation]
        KE[Key Exchange]
        KS[Key Storage]
        KR[Key Rotation]
    end
    
    subgraph "Encryption Layer"
        X25519[X25519 ECDH]
        AES[AES-256-GCM]
        ARGON[Argon2 Hashing]
    end
    
    subgraph "Transport Security"
        TLS[TLS 1.3]
        CERT[Certificate Management]
        PINNING[Certificate Pinning]
    end
    
    KG --> X25519
    KE --> X25519
    X25519 --> AES
    KS --> AES
    KR --> AES
    
    AES --> TLS
    ARGON --> TLS
    CERT --> TLS
    PINNING --> TLS
```

### Security Layers

1. **Transport Layer Security**
   - TLS 1.3 for all connections
   - Certificate pinning
   - Perfect Forward Secrecy

2. **Application Layer Security**
   - End-to-end encryption with AES-256-GCM
   - X25519 key exchange
   - Message authentication codes

3. **Authentication Security**
   - Argon2 password hashing
   - Session token management
   - Rate limiting and brute force protection

4. **Data Security**
   - Encrypted storage of sensitive data
   - Secure key derivation
   - Memory-safe operations

### Threat Model

| Threat | Mitigation |
|--------|------------|
| **Man-in-the-middle** | TLS + Certificate pinning |
| **Message interception** | End-to-end encryption |
| **Password attacks** | Argon2 + Rate limiting |
| **Session hijacking** | Secure tokens + HTTPS only |
| **Data breach** | Encrypted storage |
| **Memory attacks** | Memory-safe Rust + Zeroization |

## Transport Layer

### Transport Abstraction

```mermaid
classDiagram
    class Transport {
        <<interface>>
        +send(data: &[u8]) Result~(), TransportError~
        +receive() Result~Vec~u8~, TransportError~
        +close() Result~(), TransportError~
        +is_connected() bool
    }
    
    class TcpTransport {
        -stream: TcpStream
        -config: TcpConfig
        +new(config: TcpConfig) Self
        +connect(addr: SocketAddr) Result~(), Error~
    }
    
    class EncryptedTransport {
        -inner: Box~dyn Transport~
        -encryption: Arc~dyn EncryptionService~
        +new(transport, encryption) Self
        +perform_handshake() Result~(), Error~
    }
    
    class WebSocketTransport {
        -socket: WebSocket
        -config: WsConfig
        +new(config: WsConfig) Self
        +upgrade_from_http() Result~(), Error~
    }
    
    Transport <|-- TcpTransport
    Transport <|-- EncryptedTransport
    Transport <|-- WebSocketTransport
    
    EncryptedTransport --> Transport : wraps
```

### Protocol Stack

```
┌─────────────────────────────────────┐
│           Application Layer         │
│        (Chat Protocol)              │
├─────────────────────────────────────┤
│          Encryption Layer           │
│         (AES-256-GCM)               │
├─────────────────────────────────────┤
│          Transport Layer            │
│      (TCP/WebSocket/HTTP/2)         │
├─────────────────────────────────────┤
│           Security Layer            │
│            (TLS 1.3)                │
├─────────────────────────────────────┤
│          Network Layer              │
│             (TCP/IP)                │
└─────────────────────────────────────┘
```

## Storage Architecture

### Data Model

```mermaid
erDiagram
    USER {
        string id PK
        string username UK
        string password_hash
        string email
        datetime created_at
        datetime last_seen
        json preferences
    }
    
    ROOM {
        string id PK
        string name UK
        string description
        string room_type
        string owner_id FK
        datetime created_at
        json settings
    }
    
    MESSAGE {
        string id PK
        string room_id FK
        string sender_id FK
        text content_encrypted
        string message_type
        datetime timestamp
        json metadata
    }
    
    DIRECT_MESSAGE {
        string id PK
        string sender_id FK
        string recipient_id FK
        text content_encrypted
        datetime timestamp
        boolean is_read
        json metadata
    }
    
    ROOM_MEMBERSHIP {
        string room_id FK
        string user_id FK
        string role
        datetime joined_at
        json permissions
    }
    
    SESSION {
        string token PK
        string user_id FK
        datetime created_at
        datetime expires_at
        string client_info
    }
    
    USER ||--o{ ROOM : owns
    USER ||--o{ MESSAGE : sends
    USER ||--o{ DIRECT_MESSAGE : sends
    USER ||--o{ DIRECT_MESSAGE : receives
    USER ||--o{ ROOM_MEMBERSHIP : joins
    USER ||--o{ SESSION : has
    ROOM ||--o{ MESSAGE : contains
    ROOM ||--o{ ROOM_MEMBERSHIP : has
```

### Storage Strategy

1. **Hot Data (In-Memory/Cache)**
   - Active sessions
   - Online users
   - Recent messages
   - Connection states

2. **Warm Data (Database)**
   - User profiles
   - Room configurations
   - Message history (recent)
   - Authentication data

3. **Cold Data (File Storage)**
   - Old message archives
   - File attachments
   - Backup data
   - Logs and analytics

## Scalability Considerations

### Horizontal Scaling

```mermaid
graph TB
    subgraph "Load Balancer Layer"
        LB[Load Balancer]
    end
    
    subgraph "Application Layer"
        APP1[App Server 1]
        APP2[App Server 2]
        APP3[App Server N]
    end
    
    subgraph "Cache Layer"
        REDIS1[Redis 1]
        REDIS2[Redis 2]
        REDIS3[Redis 3]
    end
    
    subgraph "Database Layer"
        MASTER[(DB Master)]
        SLAVE1[(DB Slave 1)]
        SLAVE2[(DB Slave 2)]
    end
    
    LB --> APP1
    LB --> APP2
    LB --> APP3
    
    APP1 --> REDIS1
    APP2 --> REDIS2
    APP3 --> REDIS3
    
    APP1 --> MASTER
    APP2 --> MASTER
    APP3 --> MASTER
    
    MASTER --> SLAVE1
    MASTER --> SLAVE2
```

### Scaling Strategies

1. **Stateless Application Servers**
   - Session data in Redis
   - No server affinity required
   - Easy horizontal scaling

2. **Database Scaling**
   - Read replicas for queries
   - Sharding by user ID
   - Connection pooling

3. **Cache Scaling**
   - Redis cluster for distributed caching
   - Consistent hashing
   - Cache invalidation strategies

4. **Message Broadcasting**
   - Pub/Sub pattern for real-time updates
   - Message queues for reliability
   - WebSocket connection management

## Performance Architecture

### Performance Characteristics

| Component | Throughput | Latency | Memory |
|-----------|------------|---------|--------|
| **Message Processing** | 10K+ msg/sec | <1ms | 50MB |
| **Connection Handling** | 1K+ conn/server | <10ms | 100MB |
| **Database Operations** | 5K+ ops/sec | <5ms | 500MB |
| **Encryption/Decryption** | 1M+ ops/sec | <0.1ms | 10MB |

### Optimization Strategies

1. **Zero-Copy Operations**
   ```rust
   // Avoid unnecessary allocations
   async fn process_message(buffer: &[u8]) -> Result<(), Error> {
       // Process directly from buffer without copying
   }
   ```

2. **Connection Pooling**
   ```rust
   // Reuse connections efficiently
   struct ConnectionPool {
       pool: Vec<Connection>,
       available: VecDeque<usize>,
   }
   ```

3. **Batch Processing**
   ```rust
   // Process multiple messages together
   async fn process_batch(messages: Vec<Message>) -> Result<(), Error> {
       // Batch database operations
   }
   ```

### Monitoring and Metrics

```mermaid
graph LR
    subgraph "Application Metrics"
        MSG_RATE[Message Rate]
        CONN_COUNT[Connection Count]
        LATENCY[Response Latency]
        ERROR_RATE[Error Rate]
    end
    
    subgraph "System Metrics"
        CPU[CPU Usage]
        MEMORY[Memory Usage]
        DISK[Disk I/O]
        NETWORK[Network I/O]
    end
    
    subgraph "Business Metrics"
        ACTIVE_USERS[Active Users]
        ROOM_COUNT[Room Count]
        MSG_VOLUME[Message Volume]
        UPTIME[Service Uptime]
    end
    
    MSG_RATE --> DASHBOARD[Monitoring Dashboard]
    CONN_COUNT --> DASHBOARD
    LATENCY --> DASHBOARD
    ERROR_RATE --> DASHBOARD
    
    CPU --> DASHBOARD
    MEMORY --> DASHBOARD
    DISK --> DASHBOARD
    NETWORK --> DASHBOARD
    
    ACTIVE_USERS --> DASHBOARD
    ROOM_COUNT --> DASHBOARD
    MSG_VOLUME --> DASHBOARD
    UPTIME --> DASHBOARD
```

## Deployment Architecture

### Development Environment

```mermaid
graph TB
    subgraph "Developer Machine"
        IDE[IDE/Editor]
        CLIENT[Client Binary]
        SERVER[Server Binary]
        DB[SQLite DB]
    end
    
    IDE --> CLIENT
    IDE --> SERVER
    CLIENT <--> SERVER
    SERVER <--> DB
```

### Production Environment

```mermaid
graph TB
    subgraph "Internet"
        USERS[Users]
    end
    
    subgraph "Load Balancer"
        LB[HAProxy/Nginx]
    end
    
    subgraph "Application Servers"
        APP1[Lair Chat Server 1]
        APP2[Lair Chat Server 2]
        APP3[Lair Chat Server 3]
    end
    
    subgraph "Database Cluster"
        MASTER[(PostgreSQL Master)]
        SLAVE1[(PostgreSQL Slave 1)]
        SLAVE2[(PostgreSQL Slave 2)]
    end
    
    subgraph "Cache Cluster"
        REDIS1[Redis 1]
        REDIS2[Redis 2]
        REDIS3[Redis 3]
    end
    
    subgraph "Storage"
        S3[Object Storage]
    end
    
    subgraph "Monitoring"
        PROMETHEUS[Prometheus]
        GRAFANA[Grafana]
        ALERTS[AlertManager]
    end
    
    USERS --> LB
    LB --> APP1
    LB --> APP2
    LB --> APP3
    
    APP1 --> MASTER
    APP2 --> MASTER
    APP3 --> MASTER
    
    APP1 --> SLAVE1
    APP2 --> SLAVE2
    APP3 --> SLAVE1
    
    APP1 --> REDIS1
    APP2 --> REDIS2
    APP3 --> REDIS3
    
    APP1 --> S3
    APP2 --> S3
    APP3 --> S3
    
    APP1 --> PROMETHEUS
    APP2 --> PROMETHEUS
    APP3 --> PROMETHEUS
    
    PROMETHEUS --> GRAFANA
    PROMETHEUS --> ALERTS
```

### Container Architecture

```dockerfile
# Multi-stage build for optimal image size
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/lair-chat-server /usr/local/bin/
EXPOSE 8080
CMD ["lair-chat-server"]
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: lair-chat-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: lair-chat-server
  template:
    metadata:
      labels:
        app: lair-chat-server
    spec:
      containers:
      - name: server
        image: lair-chat:latest
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-credentials
              key: url
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

---

## Future Architecture Considerations

### Planned Enhancements

1. **Microservices Architecture**
   - Separate authentication service
   - Dedicated message routing service
   - Independent scaling per service

2. **Event-Driven Architecture**
   - Event sourcing for message history
   - CQRS for read/write separation
   - Event streaming with Apache Kafka

3. **Multi-Region Deployment**
   - Geographic distribution
   - Data replication strategies
   - Conflict resolution mechanisms

4. **Advanced Features**
   - Voice/video calling integration
   - File sharing and storage
   - Plugin architecture
   - Mobile client support

### Technology Evolution

- **WebAssembly**: For client-side plugins
- **gRPC**: For high-performance service communication
- **GraphQL**: For flexible API queries
- **Kubernetes**: For container orchestration
- **Service Mesh**: For advanced networking and security

This architecture documentation provides a comprehensive overview of the current system design and serves as a foundation for future development and scaling decisions.

---

*Architecture documentation last updated: June 2025*