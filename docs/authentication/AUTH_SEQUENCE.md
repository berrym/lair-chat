# Authentication Sequence Details

## 1. Initial Connection and Key Exchange

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server
    
    C->>S: TCP Connection Request
    S->>C: Connection Accept
    
    Note over C,S: Diffie-Hellman Key Exchange
    C->>C: Generate Ephemeral Key Pair
    S->>S: Generate Ephemeral Key Pair
    S->>C: Server Public Key
    C->>S: Client Public Key
    
    Note over C,S: Both sides compute shared secret
    C->>C: Compute Shared Secret
    S->>S: Compute Shared Secret
```

## 2. User Registration

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server
    participant DB as Database
    
    C->>S: Registration Request (username)
    S->>DB: Check Username Availability
    
    alt Username Available
        DB-->>S: Username Free
        S->>S: Generate Salt
        S->>C: Send Salt + Registration Challenge
        
        C->>C: Generate Password Hash
        Note over C: Argon2id(password + salt)
        
        C->>S: Send Registration Data
        Note over S: Validate Data Format
        
        S->>DB: Store User Data
        DB-->>S: Success
        S->>C: Registration Complete
    else Username Taken
        DB-->>S: Username Exists
        S->>C: Username Unavailable
    end
```

## 3. User Authentication

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server
    participant DB as Database
    
    C->>S: Auth Request (username)
    S->>DB: Fetch User Data
    
    alt User Found
        DB-->>S: User Data (salt, hash)
        S->>S: Generate Auth Challenge
        S->>C: Send Challenge + Salt
        
        C->>C: Generate Auth Proof
        Note over C: Compute proof using password
        
        C->>S: Send Auth Proof
        
        alt Auth Success
            S->>S: Generate Session Token
            S->>C: Send Session Token
            Note over C,S: Secure Channel Established
        else Auth Failure
            S->>C: Auth Failed
            Note over S: Update Failed Attempts
        end
    else User Not Found
        DB-->>S: No User
        S->>C: Invalid Username
    end
```

## 4. Session Management

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server
    participant Cache as Session Cache
    
    Note over C,S: Active Session
    
    C->>S: Request with Session Token
    S->>Cache: Validate Token
    
    alt Valid Token
        Cache-->>S: Token Valid
        S->>C: Process Request
        
        alt Token Near Expiry
            S->>S: Generate New Token
            S->>Cache: Update Session
            S->>C: Include New Token
        end
    else Invalid Token
        Cache-->>S: Token Invalid
        S->>C: Authentication Required
    end
```

## 5. Password Reset Flow

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server
    participant DB as Database
    
    C->>S: Password Reset Request
    S->>DB: Verify User Exists
    
    alt User Found
        S->>S: Generate Reset Token
        S->>DB: Store Reset Token
        S->>C: Send Reset Instructions
        
        Note over C: User confirms reset
        
        C->>S: Reset Confirmation
        S->>S: Generate New Salt
        S->>C: Send New Salt
        
        C->>C: Generate New Password Hash
        C->>S: Send New Credentials
        
        S->>DB: Update Credentials
        S->>C: Reset Complete
    else User Not Found
        S->>C: Invalid Username
    end
```

## 6. Rate Limiting

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server
    participant RL as Rate Limiter
    
    C->>S: Authentication Request
    S->>RL: Check Rate Limit
    
    alt Within Limit
        RL-->>S: Allowed
        Note over C,S: Normal Auth Flow
    else Rate Limited
        RL-->>S: Limited
        S->>C: Too Many Attempts
        Note over C: Exponential Backoff
    end
```

## Technical Details

### Authentication Proof Generation

```rust
struct AuthProof {
    username: String,
    timestamp: u64,
    nonce: [u8; 32],
    proof: [u8; 32],
}

impl AuthProof {
    fn generate(
        username: &str,
        password: &str,
        salt: &[u8],
        challenge: &Challenge
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        let nonce = rand::thread_rng()
            .gen::<[u8; 32]>();
            
        let proof = argon2::hash_password(
            password.as_bytes(),
            salt,
            &argon2::Config::default()
        );
        
        Self {
            username: username.to_string(),
            timestamp,
            nonce,
            proof: proof.unwrap(),
        }
    }
}
```

### Session Token Structure

```rust
struct SessionToken {
    // Token ID
    id: Uuid,
    
    // User information
    user_id: Uuid,
    username: String,
    
    // Timing information
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    
    // Security metadata
    fingerprint: String,
    permissions: Vec<Permission>,
    
    // Cryptographic proof
    signature: [u8; 64],
}
```

### Rate Limiting Configuration

```rust
struct RateLimitConfig {
    // Window size in seconds
    window_size: u64,
    
    // Maximum attempts per window
    max_attempts: u32,
    
    // Backoff multiplier for consecutive failures
    backoff_multiplier: f64,
    
    // Maximum backoff time in seconds
    max_backoff: u64,
    
    // Separate limits for different operations
    limits: HashMap<Operation, RateLimit>,
}
```

## Error Handling

| Error Code | Description | Action |
|------------|-------------|---------|
| AUTH001 | Invalid Credentials | Increment failed attempts |
| AUTH002 | Rate Limited | Apply backoff |
| AUTH003 | Account Locked | Require recovery |
| AUTH004 | Session Expired | Request reauthentication |
| AUTH005 | Invalid Token | Clear session |
| AUTH006 | Server Error | Retry with backoff |

## Security Considerations

1. **Timing Attacks**
   - Use constant-time comparisons
   - Add random delays
   - Normalize response times

2. **Replay Protection**
   - Include timestamps in proofs
   - Use unique nonces
   - Maintain nonce history

3. **Session Security**
   - Rotate tokens regularly
   - Validate client fingerprints
   - Monitor for suspicious activity

4. **Data Protection**
   - Encrypt sensitive data
   - Use secure key storage
   - Implement secure erasure