# Invitation System Debug and Fix Guide

## Current Issue Summary

The `/invite` command has two critical problems:
1. **Inviter sees raw formatted text** instead of proper confirmation
2. **Invitee never receives invitations** - messages not reaching recipients

## Debug Steps to Identify Root Cause

### Step 1: Enable Debug Logging
```bash
# Run both server and clients with debug logging
RUST_LOG=debug cargo run --bin lair-chat-server 2>&1 | tee server_debug.log &
RUST_LOG=debug cargo run --bin lair-chat-client 2>&1 | tee alice_debug.log &
RUST_LOG=debug cargo run --bin lair-chat-client 2>&1 | tee bob_debug.log &
```

### Step 2: Test Invitation Flow
1. Alice authenticates as "alice"
2. Bob authenticates as "bob" 
3. Alice types: `/invite bob testroom`
4. Check what Alice sees (should see confirmation, not raw text)
5. Check what Bob sees (should see invitation notification)

### Step 3: Check Debug Logs

Look for these debug messages in the logs:

**In Alice's log (inviter):**
```
🔍 DEBUG: INVITATION MESSAGE DETECTED: 'You invited bob to join room 'testroom''
🔍 DEBUG: INVITATION SENT PATTERN MATCHED
🔍 MESSAGE_ROUTER: Successfully parsed invitation confirmation to 'bob' for room 'testroom'
🔍 DEBUG: About to route InvitationSent message
🔍 DEBUG: InvitationSent routing result: Ok(())
🔍 DEBUG: Processing InvitationSent to 'bob' for room 'testroom'
🔍 DEBUG: Sending DisplayMessage: '📤 Invitation sent to bob for room 'testroom''
```

**In Bob's log (invitee):**
```
🔍 DEBUG: INVITATION MESSAGE DETECTED: 'alice invited you to join room 'testroom''
🔍 DEBUG: INVITATION RECEIVED PATTERN MATCHED
🔍 MESSAGE_ROUTER: Successfully parsed invitation from 'alice' for room 'testroom'
🔍 DEBUG: About to route InvitationReceived message
🔍 DEBUG: InvitationReceived routing result: Ok(())
🔍 DEBUG: Processing InvitationReceived from 'alice' for room 'testroom'
```

## Possible Root Causes & Fixes

### Issue 1: Message Router Parsing Failure

**Symptoms:** Debug logs show parsing failures or no invitation patterns matched

**Cause:** The server message format doesn't match parsing patterns

**Check:** Look for these in logs:
- `🔍 DEBUG: Failed to find closing quote in room name`
- `🔍 DEBUG: Failed to find inviter end`
- `Message router failed to process`

**Fix:** Adjust parsing patterns in `message_router.rs` to match exact server format

### Issue 2: Message Router Success But Legacy Processing Also Runs

**Symptoms:** Debug logs show successful parsing but raw text still displayed

**Cause:** Message router succeeds but early return isn't working

**Check:** Look for both:
- `✅ Message router successfully processed message`
- Raw text still appearing in UI

**Fix:** Ensure early return is working properly in `app.rs` message processing

### Issue 3: Action Handling Failure

**Symptoms:** Message router succeeds but actions not processed

**Cause:** `InvitationReceived` or `InvitationSent` actions not handled properly

**Check:** Look for:
- `🔍 DEBUG: InvitationSent DisplayMessage send result: Err(...)`
- Action routing failures

**Fix:** Check action handlers in `app.rs`

### Issue 4: Server Not Sending Messages

**Symptoms:** No invitation messages in recipient logs at all

**Cause:** Server failing to deliver messages to target users

**Check:** Server logs for:
- `📤 Processing invitation from alice to bob for room 'testroom'`
- User lookup failures

**Fix:** Check server user presence and message delivery

## Quick Diagnostic Commands

### Check if server is sending invitations:
```bash
grep "invited you to join" server_debug.log
grep "You invited" server_debug.log
```

### Check if message router is processing:
```bash
grep "INVITATION.*PATTERN MATCHED" alice_debug.log bob_debug.log
grep "Successfully parsed invitation" alice_debug.log bob_debug.log
```

### Check if actions are being sent:
```bash
grep "InvitationReceived action" bob_debug.log
grep "InvitationSent.*DisplayMessage" alice_debug.log
```

## Expected Working Flow

### Server Side (in server_debug.log):
```
📤 Processing invitation from alice to bob for room 'testroom'
Sending invitation to bob: SYSTEM_MESSAGE:alice invited you to join room 'testroom'
Sending confirmation to alice: SYSTEM_MESSAGE:You invited bob to join room 'testroom'
```

### Alice Side (inviter):
```
Received: SYSTEM_MESSAGE:You invited bob to join room 'testroom'
Pattern matched → InvitationSent message
Displays: 📤 Invitation sent to bob for room 'testroom'
```

### Bob Side (invitee):
```
Received: SYSTEM_MESSAGE:alice invited you to join room 'testroom'
Pattern matched → InvitationReceived message
Triggers: InvitationReceived action → invitation UI
```

## Manual Fix Steps

### If Message Router Parsing Fails:
1. Check exact server message format in logs
2. Adjust parsing patterns in `handle_invitation_message()`
3. Test pattern matching with exact strings

### If Legacy Processing Interferes:
1. Verify early return after successful message router processing
2. Check that SYSTEM_MESSAGE: is properly excluded from legacy filtering
3. Add more specific filtering if needed

### If Action Handling Fails:
1. Check `InvitationReceived` action handler in `app.rs`
2. Verify action channel is working
3. Test action flow with simple debug messages

### If Server Delivery Fails:
1. Check user lookup in server
2. Verify both users are connected and authenticated
3. Test with simple system messages first

## Testing Verification

After fixes, verify:
1. **Alice types `/invite bob testroom`**
2. **Alice sees:** `📤 Invitation sent to bob for room 'testroom'` (NO raw text)
3. **Bob sees:** Proper invitation notification UI
4. **Bob can accept/decline invitation**
5. **No `SYSTEM_MESSAGE:` text visible to either user**

## Common Fixes Applied

Based on DM and room fixes, the solution likely involves:
1. Ensuring all `SYSTEM_MESSAGE:` content goes through message router
2. Early returns prevent legacy processing 
3. Proper action routing for invitation UI
4. Formatted display messages replace raw protocol text

The invitation system should work exactly like the fixed DM and room systems - no raw protocol messages visible, proper UI interactions, immediate user feedback.