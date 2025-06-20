# Manual Invitation Test Guide

## Purpose
This guide helps you manually test the invitation system to identify the exact issue with `/invite user roomname` not displaying anything to the invited user.

## Prerequisites
1. Make sure the server is running: `cargo run --bin lair-chat-server`
2. Have two terminal windows ready for two different clients

## Test Scenario

### Setup (Terminal 1 - Alice)
1. Start the client: `cargo run --bin lair-chat-client`
2. Login as Alice:
   - Username: `alice`
   - Password: `password123`
3. Create a room: `/create-room testroom`
4. Verify you're in the room (should see confirmation)

### Setup (Terminal 2 - Bob)
1. Start the client: `cargo run --bin lair-chat-client`
2. Login as Bob:
   - Username: `bob`
   - Password: `password123`
3. Stay in the lobby (don't join any rooms yet)

### Test Invitation Flow

#### Step 1: Alice sends invitation
In Alice's terminal, type:
```
/invite bob testroom
```

**Expected behavior:**
- Alice should see: `📤 Invitation sent to bob for room 'testroom'` or similar confirmation
- If Alice sees an error, note exactly what it says

#### Step 2: Check Bob's terminal
**Expected behavior:**
- Bob should see:
  ```
  🔔 INVITATION: alice invited you to join room 'testroom'
  💡 To respond: '/accept testroom' or '/decline testroom' or just '/accept' for latest
     You can also use '/join testroom' to accept or '/invites' to see all pending
  ```

**If Bob sees nothing:** This confirms the bug

#### Step 3: Verify /join works
In Bob's terminal, type:
```
/join testroom
```

**Expected behavior:**
- Bob should successfully join the room
- This confirms the room exists and Bob can join it manually

## Debugging Steps

### Check 1: Room Membership
After Alice creates the room, verify she's actually in it:
- Alice should see her current room displayed in the status bar
- Alice should be able to send messages to the room

### Check 2: User Validation
Try inviting a non-existent user:
```
/invite nonexistentuser testroom
```
Alice should see an error message about the user not being found.

### Check 3: Room Validation
Try inviting to a non-existent room:
```
/invite bob fakeroom
```
Alice should see an error message about the room not existing.

### Check 4: Permission Validation
1. Have Alice leave the room: `/leave`
2. Try to invite from outside the room: `/invite bob testroom`
3. Alice should see an error about needing to be in the room to invite others

## Common Issues to Check

### Issue 1: Authentication Problems
- Verify both users can log in successfully
- Check that usernames `alice` and `bob` exist (they should be created by default)
- Verify password is `password123`

### Issue 2: Room Creation Problems
- Verify the room was actually created
- Check if Alice is actually in the room after creation
- Try using `/rooms` to list available rooms

### Issue 3: Network/Connection Issues
- Check if both clients are connected to the server
- Look for any connection error messages
- Verify server is running and accessible

### Issue 4: Command Processing Issues
- Try other commands like `/help` to verify command processing works
- Check if `/invite` command is recognized (should show usage if typed without arguments)

## Log Analysis

### Server Logs
Look for these patterns in server logs:
- `INVITE_USER:bob:testroom` - Server received the invite command
- `Processing invitation from alice to bob for room 'testroom'` - Server is processing
- `send_to_user` calls - Server attempting to send to Bob
- Any error messages about room validation

### Client Logs (if running with RUST_LOG=debug)
Look for these patterns:
- `Action::SendMessage("INVITE_USER:bob:testroom")` - Command was processed
- `SYSTEM_MESSAGE:` entries - Server responses
- `InvitationReceived` actions - Client processing invitations

## Expected Results

### If Working Correctly:
1. Alice sees confirmation of invitation sent
2. Bob immediately sees invitation notification with response options
3. Bob can accept/decline or join manually

### If Bug Exists:
1. Alice may or may not see confirmation
2. Bob sees nothing about the invitation
3. Bob can still join manually with `/join testroom`

## Next Steps Based on Results

### If Alice doesn't see confirmation:
- Issue is in command processing or server-side validation
- Check authentication, room membership, command routing

### If Alice sees confirmation but Bob sees nothing:
- Issue is in server-to-client message delivery or client-side parsing
- Check network connection, message routing, invitation parsing

### If Bob sees invitation but it's malformed:
- Issue is in client-side message formatting
- Check message router display logic

## Quick Verification Commands

Test these commands to verify system components:

```bash
# Alice's commands
/help                    # Verify command processing
/rooms                   # Verify room listing works
/create-room testroom    # Create test room
/invite                  # Should show usage error
/invite bob testroom     # The actual test

# Bob's commands  
/help                    # Verify command processing
/rooms                   # Should see testroom in list
/join testroom           # Should work if room exists
```

## Report Template

When reporting results, include:

1. **Exact steps taken**
2. **What Alice saw** (copy exact messages)
3. **What Bob saw** (copy exact messages)
4. **Any error messages**
5. **Whether /join worked for Bob**
6. **Server log snippets** (if available)

This will help identify exactly where in the invitation flow the issue occurs.