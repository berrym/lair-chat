# TCP Server Database Migration Strategy

## Executive Summary

This document outlines a comprehensive strategy for migrating the Lair Chat TCP server from its current in-memory data storage to using the same `StorageManager` and database as the REST API server. This migration will achieve true data integration, persistence, and consistency between both server components.

## Current State Analysis

### TCP Server (Current)
- **Storage**: In-memory HashMap structures
- **Persistence**: None (data lost on restart)
- **Authentication**: `MemoryUserStorage` and `MemorySessionStorage`
- **Data Models**: Simple structs without database mapping
- **Integration**: None with REST API data

### REST API Server (Target State)
- **Storage**: SQLite database via `StorageManager`
- **Persistence**: Full database persistence with migrations
- **Authentication**: Database-backed user and session storage
- **Data Models**: Comprehensive models with relationships
- **Integration**: Designed for multi-client access

## Migration Objectives

1. **Data Persistence**: All TCP operations persist across server restarts
2. **Data Integration**: TCP and REST API share identical data
3. **Consistency**: Eliminate duplicate data structures and logic
4. **Reliability**: Database transactions and error handling
5. **Performance**: Maintain TCP server real-time performance
6. **Backward Compatibility**: Preserve existing TCP protocol

## Implementation Strategy

### Phase 1: Infrastructure Setup (1-2 days)

#### Step 1.1: Update Dependencies
- [ ] Verify `Cargo.toml` includes all storage dependencies
- [ ] Ensure compatibility between TCP and REST storage versions
- [ ] Add any missing database-related dependencies

#### Step 1.2: Shared Storage Initialization
- [ ] Modify `main()` function to create single `StorageManager` instance
- [ ] Pass same storage instance to both TCP and REST servers
- [ ] Ensure database migrations run before either server starts

**Implementation:**
```rust
// In src/bin/server.rs main() function
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize SHARED database configuration
    let database_config = DatabaseConfig {
        url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/lair_chat.db".to_string()),
        max_connections: 20, // Increase for dual server access
        min_connections: 2,
        connection_timeout: Duration::from_secs(30),
        idle_timeout: Duration::from_secs(300),
        auto_migrate: true,
    };

    // Create SINGLE StorageManager instance
    let storage = Arc::new(
        StorageManager::new(database_config)
            .await
            .expect("Failed to initialize shared storage manager"),
    );

    // TCP server uses shared storage
    let tcp_state = Arc::new(Mutex::new(SharedState::new(Arc::clone(&storage))));
    
    // REST API uses same shared storage
    let api_state = ApiState::new(Arc::clone(&storage), jwt_secret, Arc::new(config));

    // Both servers now share the same database
    // ... rest of server startup
}
```

### Phase 2: Data Structure Migration (2-3 days)

#### Step 2.1: Modify SharedState Structure
- [ ] Replace in-memory data structures with storage references
- [ ] Keep connection-specific data (peers, connected_users)
- [ ] Remove redundant data structures

**Before:**
```rust
pub struct SharedState {
    peers: HashMap<SocketAddr, WriteData>,
    auth_service: Arc<AuthService>,
    connected_users: HashMap<String, ConnectedUser>,
    rooms: HashMap<String, Room>,                    // REMOVE
    pending_invitations: HashMap<String, Vec<PendingInvitation>>, // REMOVE
    start_time: std::time::Instant,
}
```

**After:**
```rust
pub struct SharedState {
    peers: HashMap<SocketAddr, WriteData>,
    storage: Arc<StorageManager>,                    // ADD
    connected_users: HashMap<String, ConnectedUser>, // KEEP (connection state)
    start_time: std::time::Instant,
}
```

#### Step 2.2: Update ConnectedUser Model
- [ ] Map TCP `ConnectedUser` to database `User` model
- [ ] Add user_id field for database references
- [ ] Maintain connection-specific information

```rust
#[derive(Debug, Clone)]
struct ConnectedUser {
    user_id: String,        // Database user ID
    username: String,       // Cache for performance
    address: SocketAddr,    // Connection-specific
    connected_at: u64,      // Connection-specific
    current_room_id: Option<String>, // Database room ID
}
```

#### Step 2.3: Create Helper Functions
- [ ] User ID lookup functions
- [ ] Room ID resolution functions
- [ ] Database error handling utilities

```rust
impl SharedState {
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, StorageError> {
        self.storage.users().get_user_by_username(username).await
    }

    async fn get_room_by_name(&self, room_name: &str) -> Result<Option<Room>, StorageError> {
        self.storage.rooms().get_room_by_name(room_name).await
    }

    async fn get_user_rooms(&self, user_id: &str) -> Result<Vec<Room>, StorageError> {
        self.storage.rooms().get_user_rooms(user_id).await
    }
}
```

### Phase 3: Authentication Migration (2-3 days)

#### Step 3.1: Replace Memory-Based Authentication
- [ ] Remove `MemoryUserStorage` and `MemorySessionStorage`
- [ ] Use database-backed authentication directly
- [ ] Implement JWT token generation for TCP sessions

#### Step 3.2: Update Login/Registration Logic
- [ ] Replace `auth_service` calls with direct storage calls
- [ ] Add proper password hashing validation
- [ ] Create database sessions for TCP connections

**Implementation:**
```rust
async fn handle_tcp_authentication(
    storage: &StorageManager,
    username: &str,
    password: &str,
    is_registration: bool,
) -> Result<User, AuthError> {
    if is_registration {
        // Check if user exists
        if let Some(_) = storage.users().get_user_by_username(username).await? {
            return Err(AuthError::UsernameTaken);
        }

        // Create new user
        let password_hash = hash_password(password)?;
        let user = User {
            id: generate_id(),
            username: username.to_string(),
            password_hash,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            is_active: true,
            role: UserRole::User,
            // ... other required fields
        };

        storage.users().create_user(user).await
    } else {
        // Login existing user
        let user = storage.users()
            .get_user_by_username(username).await?
            .ok_or(AuthError::UserNotFound)?;

        verify_password(password, &user.password_hash)?;
        Ok(user)
    }
}
```

#### Step 3.3: Session Management
- [ ] Create TCP sessions in database
- [ ] Track active TCP connections
- [ ] Handle session cleanup on disconnect

### Phase 4: Room Operations Migration (3-4 days)

#### Step 4.1: Room Creation
- [ ] Replace in-memory room creation with database operations
- [ ] Add proper room validation
- [ ] Create room membership records

```rust
async fn handle_create_room(
    storage: &StorageManager,
    creator_user_id: &str,
    room_name: &str,
) -> Result<Room, StorageError> {
    // Validate room name
    if room_name.is_empty() || room_name.to_lowercase() == "lobby" {
        return Err(StorageError::ValidationError {
            field: "name".to_string(),
            message: "Invalid room name".to_string(),
        });
    }

    // Check if room exists
    if storage.rooms().room_name_exists(room_name).await? {
        return Err(StorageError::DuplicateError {
            entity: "Room".to_string(),
            message: format!("Room '{}' already exists", room_name),
        });
    }

    // Create room
    let room = Room {
        id: generate_id(),
        name: room_name.to_string(),
        display_name: room_name.to_string(),
        description: None,
        topic: None,
        room_type: RoomType::Channel,
        privacy: RoomPrivacy::Public,
        settings: RoomSettings::default(),
        created_by: creator_user_id.to_string(),
        created_at: current_timestamp(),
        updated_at: current_timestamp(),
        is_active: true,
    };

    let created_room = storage.rooms().create_room(room).await?;

    // Add creator as owner
    storage.rooms().add_member(
        &created_room.id,
        creator_user_id,
        RoomRole::Owner,
    ).await?;

    Ok(created_room)
}
```

#### Step 4.2: Room Joining/Leaving
- [ ] Replace in-memory user tracking with database membership
- [ ] Update room membership tables
- [ ] Handle room permission checks

#### Step 4.3: Room Listing and Status
- [ ] Replace in-memory room queries with database queries
- [ ] Implement efficient room member lookups
- [ ] Add room activity tracking

### Phase 5: Message Handling Migration (2-3 days)

#### Step 5.1: Message Persistence
- [ ] Store all TCP messages in database
- [ ] Add message metadata and timestamps
- [ ] Implement message history retrieval

```rust
async fn handle_tcp_message(
    storage: &StorageManager,
    room_id: &str,
    user_id: &str,
    content: &str,
) -> Result<Message, StorageError> {
    let message = Message {
        id: generate_id(),
        room_id: room_id.to_string(),
        user_id: user_id.to_string(),
        content: content.to_string(),
        message_type: MessageType::Text,
        timestamp: current_timestamp(),
        edited_at: None,
        parent_message_id: None,
        metadata: MessageMetadata::default(),
        is_deleted: false,
        deleted_at: None,
    };

    storage.messages().create_message(message).await
}
```

#### Step 5.2: Message Broadcasting
- [ ] Keep real-time TCP broadcasting for connected users
- [ ] Store messages before broadcasting
- [ ] Add message delivery confirmation

#### Step 5.3: Message History
- [ ] Implement message history commands
- [ ] Add pagination for large histories
- [ ] Support message search capabilities

### Phase 6: Invitation System Migration (1-2 days)

#### Step 6.1: Database-Backed Invitations
- [ ] Replace in-memory invitation tracking
- [ ] Add invitation expiration handling
- [ ] Implement invitation cleanup

#### Step 6.2: Invitation Operations
- [ ] Create, accept, reject, and list invitations
- [ ] Add invitation notification system
- [ ] Handle invitation permissions

### Phase 7: Error Handling and Validation (1-2 days)

#### Step 7.1: Database Error Handling
- [ ] Map `StorageError` to TCP protocol responses
- [ ] Add comprehensive error logging
- [ ] Implement graceful degradation

#### Step 7.2: Transaction Management
- [ ] Wrap multi-step operations in transactions
- [ ] Add rollback capabilities
- [ ] Handle concurrent access issues

```rust
async fn handle_room_operation_with_transaction(
    storage: &StorageManager,
    operation: impl Fn(&mut Transaction) -> BoxFuture<Result<(), StorageError>>,
) -> Result<(), StorageError> {
    let mut tx = storage.begin_transaction().await?;
    
    match operation(&mut tx).await {
        Ok(_) => {
            tx.commit().await?;
            Ok(())
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}
```

### Phase 8: Testing and Validation (2-3 days)

#### Step 8.1: Unit Tests
- [ ] Test database operations in isolation
- [ ] Mock storage for connection logic tests
- [ ] Verify error handling paths

#### Step 8.2: Integration Tests
- [ ] Test TCP and REST API data consistency
- [ ] Verify real-time operations with persistence
- [ ] Test concurrent access scenarios

#### Step 8.3: Performance Testing
- [ ] Benchmark database operation performance
- [ ] Test under high connection loads
- [ ] Optimize slow queries

### Phase 9: Deployment and Migration (1-2 days)

#### Step 9.1: Data Migration Script
- [ ] Create script to migrate existing in-memory data (if any)
- [ ] Backup existing database
- [ ] Test migration rollback procedures

#### Step 9.2: Configuration Updates
- [ ] Update environment variables
- [ ] Adjust database connection pools
- [ ] Configure logging for database operations

#### Step 9.3: Monitoring Setup
- [ ] Add database performance monitoring
- [ ] Track TCP server database usage
- [ ] Set up alerts for database issues

## Implementation Timeline

| Phase | Duration | Dependencies | Deliverables |
|-------|----------|-------------|-------------|
| 1: Infrastructure | 1-2 days | None | Shared storage setup |
| 2: Data Structures | 2-3 days | Phase 1 | Updated SharedState |
| 3: Authentication | 2-3 days | Phase 2 | Database auth |
| 4: Room Operations | 3-4 days | Phase 3 | Room CRUD operations |
| 5: Message Handling | 2-3 days | Phase 4 | Message persistence |
| 6: Invitations | 1-2 days | Phase 4 | Invitation system |
| 7: Error Handling | 1-2 days | Phases 3-6 | Robust error handling |
| 8: Testing | 2-3 days | Phase 7 | Test suite |
| 9: Deployment | 1-2 days | Phase 8 | Production ready |

**Total Estimated Duration: 13-22 days**

## Risk Management

### High-Risk Areas
1. **Connection State Management**: Mixing stateful TCP connections with stateless database
2. **Performance Impact**: Database I/O affecting real-time TCP performance
3. **Concurrent Access**: Multiple servers accessing same database
4. **Data Consistency**: Ensuring TCP and REST see identical data

### Mitigation Strategies
1. **Hybrid Approach**: Keep connection state in memory, persist data operations
2. **Connection Pooling**: Optimize database connections for performance
3. **Transaction Isolation**: Use appropriate isolation levels
4. **Comprehensive Testing**: Extensive integration testing before deployment

### Rollback Plan
1. **Feature Flags**: Ability to disable database integration
2. **Fallback Mode**: Temporary in-memory mode if database fails
3. **Database Backup**: Automated backups before major changes
4. **Version Control**: Tagged releases for easy rollback

## Success Criteria

### Functional Requirements
- [ ] All TCP operations persist to database
- [ ] TCP and REST API show identical data
- [ ] No TCP protocol changes required
- [ ] Message history survives server restarts
- [ ] Admin dashboard shows live TCP data

### Performance Requirements
- [ ] TCP response time < 100ms for room operations
- [ ] Message throughput unchanged or improved
- [ ] Database connections efficiently managed
- [ ] Memory usage reduced (no large in-memory structures)

### Quality Requirements
- [ ] 100% test coverage for database operations
- [ ] Zero data loss during migration
- [ ] Comprehensive error handling
- [ ] Production monitoring and alerting

## Post-Migration Benefits

1. **Data Persistence**: Chat history, rooms, and users survive restarts
2. **True Integration**: TCP and REST API share real-time data
3. **Admin Visibility**: Dashboard shows actual TCP server activity
4. **API Access**: REST clients can access TCP chat history
5. **Scalability**: Database can handle multiple server instances
6. **Reliability**: Database transactions ensure data consistency
7. **Analytics**: Rich data for usage analytics and reporting
8. **Backup/Recovery**: Standard database backup procedures apply

## Conclusion

This migration strategy transforms the TCP server from a simple in-memory chat system to a robust, persistent, integrated component of the Lair Chat platform. The phased approach minimizes risk while ensuring thorough testing and validation at each step.

The end result will be a unified chat platform where TCP real-time connections and REST API operations work seamlessly together, sharing the same data and providing a consistent experience across all client types.

---

**Document Version**: 1.0  
**Created**: 2024  
**Author**: Lair Chat Development Team  
**Status**: Ready for Implementation