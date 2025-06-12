# Connection Key Behavior Implementation

## Overview

This document describes the implementation of connection key behavior for the Lair Chat application, specifically how the 'c' key and F2 function key handle connection states.

## Requirements Implemented

The implementation ensures that:

1. **When Connected**: Pressing 'c' or F2 should display a message informing the user they are already connected and need to disconnect first to start a new connection.

2. **When Disconnected**: Pressing 'c' or F2 should bring up the login page without requiring an application restart.

## Implementation Details

### Key Handling Logic

The key handling is implemented in `src/client/components/home.rs` in the `handle_key_event` method for both Normal and Insert modes.

#### Connection State Check

```rust
if self.connection_status == ConnectionStatus::CONNECTED {
    show_info("Already connected to server. To start a new connection, you need to disconnect first (press 'd').");
    return Ok(Some(Action::Update));
}
// If disconnected, trigger reconnection which goes back to login
Action::Reconnect
```

### Action Flow

When disconnected, the keys trigger `Action::Reconnect` which:

1. **Disconnects** any existing connection via ConnectionManager
2. **Resets auth state** to `AuthState::Unauthenticated`
3. **Updates UI components** to reflect the disconnected state
4. **Sets mode** to `Mode::Authentication`
5. **Returns to login screen** without requiring application restart

### Key Mappings

- **'c' key**: Connect/Reconnect functionality (available in both Normal and Insert modes)
- **F2 key**: Connect/Reconnect functionality (available in both Normal and Insert modes)
- **'d' key**: Disconnect functionality (when connected) - **FIXED**: Now properly disconnects using modern ConnectionManager

### User Messages

#### When Connected
```
"Already connected to server. To start a new connection, you need to disconnect first (press 'd')."
```

#### When Disconnecting for Reconnection
```
"Disconnected. Please log in again."
```

## Code Location

### Primary Implementation
- **File**: `src/client/components/home.rs`
- **Method**: `handle_key_event`
- **Lines**: ~773-825 (Normal mode), ~812-822 (Insert mode)

### Action Handling
- **File**: `src/client/app.rs`
- **Method**: `update`
- **Action**: `Action::Reconnect` (lines ~395-412)
- **Action**: `Action::DisconnectClient` (lines ~681-707) - **NEW**: Proper disconnect handling using modern ConnectionManager

### UI Drawing Logic
- **File**: `src/client/app.rs`
- **Method**: `draw`
- **Logic**: Shows login screen when `AuthState::Unauthenticated`

## State Management

### Connection Status
The implementation uses `self.connection_status` which is synchronized with the modern ConnectionManager to determine the current connection state.

### Authentication State
The implementation manages authentication state through:
- `AuthState::Unauthenticated` - Shows login screen
- `AuthState::Authenticating` - Shows loading screen
- `AuthState::Authenticated` - Shows main chat interface

## Testing

### Manual Testing Scenarios

1. **Connected State Test**:
   - Connect to server
   - Press 'c' or F2
   - Verify message appears: "Already connected to server..."
   - Verify no state change occurs

2. **Disconnected State Test**:
   - Start application (disconnected by default)
   - Press 'c' or F2
   - Verify login screen appears
   - Verify no restart required

3. **Reconnection Test**:
   - Connect to server
   - Press 'd' to disconnect
   - Press 'c' or F2
   - Verify login screen appears
   - Verify can reconnect without restart

## Benefits

1. **No Restart Required**: Users can reconnect without restarting the application
2. **Clear User Feedback**: Users receive clear messages about their connection state
3. **Consistent Behavior**: Both 'c' and F2 keys behave identically
4. **Mode Independence**: Works in both Normal and Insert modes
5. **Clean State Management**: Properly cleans up connection and authentication state
6. **Fixed Disconnect**: The 'd' key now properly disconnects using the modern ConnectionManager instead of legacy system

## Recent Fixes

### Disconnect Functionality Fix
- **Issue**: Pressing 'd' would show "Disconnecting from server..." but never actually disconnect
- **Root Cause**: The `Action::DisconnectClient` was only handled by the Home component using deprecated migration facade, while the UI checked modern ConnectionManager status
- **Solution**: Added proper `Action::DisconnectClient` handling in the main app using modern ConnectionManager
- **Files Changed**: 
  - `src/client/app.rs` - Added disconnect action handler
  - `src/client/components/home.rs` - Removed legacy disconnect handling
- **Result**: Disconnect now works properly and updates connection state correctly

### Login Screen Mode Switching Fix
- **Issue**: After disconnecting, the login page couldn't switch from register to login mode
- **Root Cause**: Login screen state wasn't properly reset when returning from disconnected state
- **Solution**: Added login screen state reset in disconnect handler using `handle_auth_state()`
- **Files Changed**: 
  - `src/client/app.rs` - Added login screen state reset in disconnect handler
- **Result**: Login screen mode switching (Ctrl+T) works properly after disconnect

### First Message Sending Fix for New Users
- **Issue**: Messages from newly registered users wouldn't send 
- **Root Cause**: Race condition where users could try to send messages before server-side registration/authentication was fully complete
- **Solution**: Added 200ms stabilization delays after successful login and registration before sending AuthenticationSuccess
- **Files Changed**: 
  - `src/client/app.rs` - Added delays in both login and registration handlers
- **Result**: First message sending now works reliably for newly registered and logged-in users

## Future Considerations

- Consider adding visual indicators for connection state in the UI
- Potentially add connection history or recent connections feature
- Consider adding keyboard shortcuts help display that shows these key bindings
- Monitor the stabilization delay timing - may need adjustment based on server performance
- Consider adding connection health checks before allowing message sending