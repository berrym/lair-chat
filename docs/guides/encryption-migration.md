# Lair Chat Encryption Migration Guide

## Overview

Lair Chat v0.6.1 introduces secure AES-256-GCM encryption, replacing the deprecated MD5-based encryption system. This guide helps you migrate to the new encryption system.

## ⚠️ Security Notice

**CRITICAL**: The old `ServerCompatibleEncryption` uses MD5 for key derivation, which is cryptographically broken and vulnerable to collision attacks. You should migrate to `AesGcmEncryption` immediately.

## Quick Migration

### For Application Developers

**OLD CODE (Insecure):**
```rust
use lair_chat::client::create_server_compatible_encryption;

let encryption = create_server_compatible_encryption();
connection_manager.with_encryption(encryption);
```

**NEW CODE (Secure):**
```rust
use lair_chat::client::create_aes_gcm_encryption_with_random_key;

let encryption = create_aes_gcm_encryption_with_random_key();
connection_manager.with_encryption(encryption);
```

### For Custom Encryption

**OLD CODE (Insecure):**
```rust
use lair_chat::client::ServerCompatibleEncryption;

let encryption = ServerCompatibleEncryption::new();
```

**NEW CODE (Secure):**
```rust
use lair_chat::client::AesGcmEncryption;

let encryption = AesGcmEncryption::new("your_password_here");
// OR for random key generation:
let encryption = AesGcmEncryption::from_key(AesGcmEncryption::generate_random_key());
```

## What Changed

### Security Improvements

| Component | Old (Insecure) | New (Secure) |
|-----------|----------------|--------------|
| Key Derivation | MD5 (broken) | SHA-256 + domain separation |
| Encryption | AES-256-GCM | AES-256-GCM (same) |
| Key Exchange | X25519 DH | X25519 DH (same) |
| Protocol | Server compatible | Server compatible (maintained) |

### Protocol Compatibility

✅ **Good News**: The handshake protocol is **fully backward compatible**
- Server code unchanged
- Client can connect to existing servers
- No configuration changes needed

### API Changes

- `create_server_compatible_encryption()` → **DEPRECATED**
- `create_aes_gcm_encryption_with_random_key()` → **RECOMMENDED**
- `ServerCompatibleEncryption::new()` → **DEPRECATED**
- `AesGcmEncryption::new()` → **RECOMMENDED**

## Migration Steps

### Step 1: Update Dependencies

Ensure you're using Lair Chat v0.6.1 or later:

```toml
[dependencies]
lair-chat = "0.6.1"
```

### Step 2: Update Code

Replace all instances of deprecated encryption:

```bash
# Find usage (bash/zsh)
grep -r "create_server_compatible_encryption" src/
grep -r "ServerCompatibleEncryption" src/

# Replace with secure alternatives
```

### Step 3: Test

Run your application and verify:
- Connection establishes successfully
- Messages encrypt/decrypt correctly
- No deprecation warnings in logs

### Step 4: Validate Security

Confirm you're using secure encryption:
```rust
// This should be true for secure encryption
assert!(encryption_service.is_using_secure_key_derivation());
```

## Advanced Usage

### Custom Password-Based Encryption

```rust
use lair_chat::client::AesGcmEncryption;

// Derive key from password (secure)
let encryption = AesGcmEncryption::new("your_secure_password");
connection_manager.with_encryption(Box::new(encryption));
```

### Random Key Generation

```rust
use lair_chat::client::AesGcmEncryption;

// Generate cryptographically secure random key
let key = AesGcmEncryption::generate_random_key();
let encryption = AesGcmEncryption::from_key(key);
connection_manager.with_encryption(Box::new(encryption));
```

### Factory Function (Recommended)

```rust
use lair_chat::client::create_aes_gcm_encryption_with_random_key;

// Easiest and most secure approach
let encryption = create_aes_gcm_encryption_with_random_key();
connection_manager.with_encryption(encryption);
```

## Troubleshooting

### Connection Issues

**Problem**: Client can't connect after migration
**Solution**: Verify server is using compatible encryption

```rust
// Enable debug logging
env_logger::init();

// Check handshake logs
RUST_LOG=debug cargo run
```

### Deprecation Warnings

**Problem**: Seeing deprecation warnings in console
**Solution**: Update to new encryption functions

```rust
// Remove these deprecated calls:
create_server_compatible_encryption()
ServerCompatibleEncryption::new()

// Use these instead:
create_aes_gcm_encryption_with_random_key()
AesGcmEncryption::new("password")
```

### Performance Concerns

**Problem**: Worried about performance impact
**Solution**: New encryption has similar or better performance

- AES-256-GCM hardware acceleration on modern CPUs
- SHA-256 is faster than MD5 on 64-bit systems
- Key exchange protocol unchanged

## Testing

### Unit Tests

```rust
#[test]
fn test_secure_encryption() {
    let encryption = create_aes_gcm_encryption_with_random_key();
    
    let message = "test message";
    let encrypted = encryption.encrypt("", message).unwrap();
    let decrypted = encryption.decrypt("", &encrypted).unwrap();
    
    assert_eq!(message, decrypted);
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_server_connection() {
    let mut connection_manager = ConnectionManager::new();
    let encryption = create_aes_gcm_encryption_with_random_key();
    
    connection_manager.with_encryption(encryption);
    // Test connection...
}
```

## Timeline

- **v0.6.1**: New AES-GCM encryption available, old encryption deprecated
- **v0.7.0**: Deprecated encryption will be removed (planned)
- **Migration Window**: 6+ months to update applications

## Security Benefits

### Before (Insecure)
❌ MD5 key derivation (vulnerable to collision attacks)  
❌ No domain separation (key reuse possible)  
❌ Deprecated cryptographic primitives  

### After (Secure)
✅ SHA-256 key derivation (collision-resistant)  
✅ Domain separation prevents key reuse  
✅ Modern cryptographic standards  
✅ Future-proof security model  

## Support

### Getting Help

- **Documentation**: Check API docs for detailed usage
- **Examples**: See `examples/` directory for working code
- **Tests**: Review test suite for best practices
- **Issues**: Report problems on GitHub

### Validation

Verify your migration with these checks:

```rust
// ✅ Good - using secure encryption
let encryption = create_aes_gcm_encryption_with_random_key();

// ❌ Bad - using deprecated encryption
let encryption = create_server_compatible_encryption(); // DEPRECATED!
```

## Conclusion

Migrating to AES-GCM encryption is:
- **Required** for security
- **Easy** to implement (one-line change)
- **Compatible** with existing servers
- **Future-proof** for long-term use

**Migrate today for a more secure chat experience! 🔐**