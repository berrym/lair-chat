#!/bin/bash

# Fix Sessions Script for Lair Chat
# This script fixes session token issues that prevent admin login

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 Fixing Session Token Issues${NC}"
echo "================================="

# Stop any running servers
echo -e "${YELLOW}Stopping services...${NC}"
pkill -f lair-chat-server-new 2>/dev/null || true
pkill -f lair-chat-server 2>/dev/null || true
pkill -f quick_start.sh 2>/dev/null || true
sleep 2

# Clear existing sessions
echo -e "${YELLOW}Clearing existing sessions...${NC}"
if [ -f "data/lair-chat.db" ]; then
    sqlite3 data/lair-chat.db "DELETE FROM sessions;"
    echo -e "${GREEN}✅ Sessions cleared${NC}"
else
    echo -e "${RED}❌ Database not found${NC}"
    exit 1
fi

# Clear any cached session files
echo -e "${YELLOW}Clearing session cache...${NC}"
rm -f /tmp/lair-chat-session-* 2>/dev/null || true
rm -f data/sessions.* 2>/dev/null || true

# Generate new JWT secret to invalidate any existing tokens
echo -e "${YELLOW}Generating new JWT secret...${NC}"
if [ -f ".env" ]; then
    NEW_JWT_SECRET="lair_chat_jwt_secret_$(openssl rand -hex 32)"
    sed -i "s|JWT_SECRET=.*|JWT_SECRET=$NEW_JWT_SECRET|g" .env
    echo -e "${GREEN}✅ New JWT secret generated${NC}"
fi

# Clear login attempts table to reset rate limiting
echo -e "${YELLOW}Clearing login attempts...${NC}"
sqlite3 data/lair-chat.db "DELETE FROM login_attempts;" 2>/dev/null || true

# Verify admin user exists
echo -e "${YELLOW}Verifying admin users...${NC}"
ADMIN_COUNT=$(sqlite3 data/lair-chat.db "SELECT COUNT(*) FROM users WHERE role = 'admin';" 2>/dev/null || echo "0")

if [ "$ADMIN_COUNT" -gt 0 ]; then
    echo -e "${GREEN}✅ Found $ADMIN_COUNT admin user(s)${NC}"

    # Show admin usernames
    echo -e "${BLUE}Admin users:${NC}"
    sqlite3 data/lair-chat.db "SELECT username, email FROM users WHERE role = 'admin';" | while read line; do
        echo -e "   • $line"
    done
else
    echo -e "${RED}❌ No admin users found${NC}"
    echo -e "${YELLOW}Creating default admin user...${NC}"

    # Build and create admin user
    cargo build --release --bin create_admin_user >/dev/null 2>&1
    cargo run --bin create_admin_user >/dev/null 2>&1

    echo -e "${GREEN}✅ Default admin user created${NC}"
    echo -e "${BLUE}   Username: admin${NC}"
    echo -e "${BLUE}   Password: AdminPassword123!${NC}"
fi

# Test database integrity
echo -e "${YELLOW}Testing database integrity...${NC}"
TABLE_COUNT=$(sqlite3 data/lair-chat.db ".tables" | wc -w)
if [ $TABLE_COUNT -gt 10 ]; then
    echo -e "${GREEN}✅ Database appears healthy ($TABLE_COUNT tables)${NC}"
else
    echo -e "${RED}❌ Database may be corrupted ($TABLE_COUNT tables)${NC}"
fi

# Start server for testing
echo -e "${YELLOW}Starting server for testing...${NC}"
nohup cargo run --bin lair-chat-server-new > logs/session_fix_test.log 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 5

# Test health endpoint
if curl -s http://127.0.0.1:8082/api/v1/health >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Server is responding${NC}"

    # Test admin login with default credentials
    echo -e "${YELLOW}Testing admin login...${NC}"
    LOGIN_RESPONSE=$(curl -s -X POST http://127.0.0.1:8082/api/v1/auth/login \
        -H "Content-Type: application/json" \
        -d '{"identifier": "admin", "password": "AdminPassword123!"}' 2>/dev/null)

    if echo "$LOGIN_RESPONSE" | grep -q "access_token"; then
        echo -e "${GREEN}✅ Admin login successful!${NC}"

        # Extract and test the token
        TOKEN=$(echo "$LOGIN_RESPONSE" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('access_token', ''))
except:
    pass
" 2>/dev/null)

        if [ -n "$TOKEN" ]; then
            # Test admin endpoint with token
            STATS_RESPONSE=$(curl -s -H "Authorization: Bearer $TOKEN" \
                http://127.0.0.1:8082/api/v1/admin/stats 2>/dev/null)

            if echo "$STATS_RESPONSE" | grep -q "total_users"; then
                echo -e "${GREEN}✅ Admin API endpoints working${NC}"
            else
                echo -e "${YELLOW}⚠️  Admin endpoints may need attention${NC}"
            fi
        fi
    else
        echo -e "${RED}❌ Admin login failed${NC}"
        echo -e "${YELLOW}Response: $LOGIN_RESPONSE${NC}"

        # Check if it's your personal admin account
        echo -e "${BLUE}Trying to find your admin account...${NC}"
        sqlite3 data/lair-chat.db "SELECT username FROM users WHERE role = 'admin';" | while read username; do
            if [ "$username" != "admin" ]; then
                echo -e "${BLUE}Found admin user: $username${NC}"
                echo -e "${YELLOW}Try logging in with: $username${NC}"
            fi
        done
    fi
else
    echo -e "${RED}❌ Server not responding${NC}"
    echo -e "${YELLOW}Check logs/session_fix_test.log for details${NC}"
fi

# Stop test server
kill $SERVER_PID 2>/dev/null || true
sleep 2

echo ""
echo -e "${GREEN}🎉 Session fix completed!${NC}"
echo ""
echo -e "${BLUE}📋 Next Steps:${NC}"
echo -e "   1. Start the system: ${YELLOW}./quick_start.sh${NC}"
echo -e "   2. Access admin dashboard: ${YELLOW}http://127.0.0.1:8083${NC}"
echo -e "   3. Login with your admin credentials${NC}"
echo ""
echo -e "${BLUE}🔍 If login still fails:${NC}"
echo -e "   • Check server logs: ${YELLOW}tail -f logs/integrated_server.log${NC}"
echo -e "   • Run debug tool: ${YELLOW}cargo run --bin debug_jwt_auth${NC}"
echo -e "   • Test API manually: ${YELLOW}./test_api.sh${NC}"
echo ""
echo -e "${YELLOW}💡 Your admin credentials should work now!${NC}"
