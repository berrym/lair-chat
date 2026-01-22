#!/bin/bash

# Remove Test Users Script for Lair Chat
# This script removes test users from both TCP server and database

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_banner() {
    echo -e "\n${PURPLE}============================================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}============================================================${NC}\n"
}

print_section() {
    echo -e "\n${CYAN}▶ $1${NC}"
    echo -e "${CYAN}$(printf '%.0s-' {1..50})${NC}"
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

# Function to identify test users
identify_test_users() {
    print_section "Identifying Test Users"

    if [ ! -f "data/lair-chat.db" ]; then
        print_error "Database file not found: data/lair-chat.db"
        exit 1
    fi

    print_step "Scanning database for test users..."

    # Common test user patterns
    local test_patterns="test% alice bob charlie demo% guest% user% admin test_% temp%"
    local found_users=""

    for pattern in $test_patterns; do
        local users=$(sqlite3 data/lair-chat.db "SELECT username FROM users WHERE username LIKE '$pattern' AND role != 'admin';" 2>/dev/null || echo "")
        if [ -n "$users" ]; then
            found_users="$found_users$users"$'\n'
        fi
    done

    # Also check for users with test email domains
    local test_email_users=$(sqlite3 data/lair-chat.db "SELECT username FROM users WHERE email LIKE '%@test.%' OR email LIKE '%@example.%' OR email LIKE '%@localhost%';" 2>/dev/null || echo "")
    if [ -n "$test_email_users" ]; then
        found_users="$found_users$test_email_users"$'\n'
    fi

    # Remove duplicates and empty lines
    found_users=$(echo "$found_users" | sort | uniq | grep -v "^$" || echo "")

    if [ -n "$found_users" ]; then
        print_warning "Found potential test users:"
        echo "$found_users" | while read -r user; do
            if [ -n "$user" ]; then
                echo -e "   • $user"
            fi
        done
        echo "$found_users"
    else
        print_info "No obvious test users found"
        echo ""
    fi
}

# Function to show all non-admin users
show_all_users() {
    print_section "Current Users in Database"

    print_step "Listing all users..."

    local users_output=$(sqlite3 data/lair-chat.db "SELECT username, email, role, created_at FROM users ORDER BY role DESC, username;" 2>/dev/null || echo "")

    if [ -n "$users_output" ]; then
        echo -e "${BLUE}Username${NC}\t\t${BLUE}Email${NC}\t\t\t${BLUE}Role${NC}\t${BLUE}Created${NC}"
        echo -e "$(printf '%.0s-' {1..80})"

        echo "$users_output" | while IFS='|' read -r username email role created_at; do
            local role_color=""
            case "$role" in
                admin) role_color="${RED}" ;;
                moderator) role_color="${YELLOW}" ;;
                *) role_color="${GREEN}" ;;
            esac

            local created_date=$(date -d "@$created_at" "+%Y-%m-%d %H:%M" 2>/dev/null || echo "Unknown")
            printf "%-20s %-30s ${role_color}%-10s${NC} %s\n" "$username" "$email" "$role" "$created_date"
        done
    else
        print_warning "No users found in database"
    fi
}

# Function to remove specific test users
remove_test_users() {
    local test_users="$1"

    if [ -z "$test_users" ]; then
        print_info "No test users to remove"
        return
    fi

    print_section "Removing Test Users"

    echo "$test_users" | while read -r username; do
        if [ -n "$username" ]; then
            print_step "Removing user: $username"

            # Get user details before deletion
            local user_info=$(sqlite3 data/lair-chat.db "SELECT id, email, role FROM users WHERE username = '$username';" 2>/dev/null || echo "")

            if [ -n "$user_info" ]; then
                local user_id=$(echo "$user_info" | cut -d'|' -f1)
                local user_email=$(echo "$user_info" | cut -d'|' -f2)
                local user_role=$(echo "$user_info" | cut -d'|' -f3)

                # Safety check - don't delete admin users
                if [ "$user_role" = "admin" ]; then
                    print_warning "Skipping admin user: $username"
                    continue
                fi

                # Remove user data in proper order to avoid foreign key constraints
                print_info "  Removing user sessions..."
                sqlite3 data/lair-chat.db "DELETE FROM sessions WHERE user_id = '$user_id';" 2>/dev/null || true

                print_info "  Removing user messages..."
                sqlite3 data/lair-chat.db "DELETE FROM messages WHERE user_id = '$user_id';" 2>/dev/null || true

                print_info "  Removing room memberships..."
                sqlite3 data/lair-chat.db "DELETE FROM room_memberships WHERE user_id = '$user_id';" 2>/dev/null || true

                print_info "  Removing room invitations..."
                sqlite3 data/lair-chat.db "DELETE FROM room_invites WHERE inviter_id = '$user_id' OR invitee_id = '$user_id';" 2>/dev/null || true

                print_info "  Removing audit logs..."
                sqlite3 data/lair-chat.db "DELETE FROM audit_logs WHERE user_id = '$user_id';" 2>/dev/null || true

                print_info "  Removing login attempts..."
                sqlite3 data/lair-chat.db "DELETE FROM login_attempts WHERE identifier = '$username' OR identifier = '$user_email';" 2>/dev/null || true

                print_info "  Removing user record..."
                sqlite3 data/lair-chat.db "DELETE FROM users WHERE id = '$user_id';" 2>/dev/null || true

                print_success "Removed user: $username ($user_email)"
            else
                print_warning "User not found: $username"
            fi
        fi
    done
}

# Function to clean up orphaned data
cleanup_orphaned_data() {
    print_section "Cleaning Up Orphaned Data"

    print_step "Removing orphaned sessions..."
    local orphaned_sessions=$(sqlite3 data/lair-chat.db "DELETE FROM sessions WHERE user_id NOT IN (SELECT id FROM users); SELECT changes();" 2>/dev/null || echo "0")
    print_info "Removed $orphaned_sessions orphaned sessions"

    print_step "Removing orphaned messages..."
    local orphaned_messages=$(sqlite3 data/lair-chat.db "DELETE FROM messages WHERE user_id NOT IN (SELECT id FROM users); SELECT changes();" 2>/dev/null || echo "0")
    print_info "Removed $orphaned_messages orphaned messages"

    print_step "Removing orphaned room memberships..."
    local orphaned_memberships=$(sqlite3 data/lair-chat.db "DELETE FROM room_memberships WHERE user_id NOT IN (SELECT id FROM users); SELECT changes();" 2>/dev/null || echo "0")
    print_info "Removed $orphaned_memberships orphaned memberships"

    print_step "Removing orphaned room invites..."
    local orphaned_invites=$(sqlite3 data/lair-chat.db "DELETE FROM room_invites WHERE inviter_id NOT IN (SELECT id FROM users) OR invitee_id NOT IN (SELECT id FROM users); SELECT changes();" 2>/dev/null || echo "0")
    print_info "Removed $orphaned_invites orphaned invites"

    print_step "Removing orphaned audit logs..."
    local orphaned_audit=$(sqlite3 data/lair-chat.db "DELETE FROM audit_logs WHERE user_id NOT IN (SELECT id FROM users); SELECT changes();" 2>/dev/null || echo "0")
    print_info "Removed $orphaned_audit orphaned audit logs"
}

# Function to stop TCP servers
stop_tcp_servers() {
    print_section "Stopping TCP Servers"

    print_step "Stopping lair-chat servers..."
    pkill -f lair-chat-server-new 2>/dev/null || true
    pkill -f lair-chat-server 2>/dev/null || true
    pkill -f lair-chat-client 2>/dev/null || true

    sleep 2
    print_success "TCP servers stopped"
}

# Function to vacuum database
vacuum_database() {
    print_section "Optimizing Database"

    print_step "Running VACUUM to reclaim space..."
    sqlite3 data/lair-chat.db "VACUUM;" 2>/dev/null || true

    print_step "Analyzing database statistics..."
    sqlite3 data/lair-chat.db "ANALYZE;" 2>/dev/null || true

    print_success "Database optimized"
}

# Function to show final statistics
show_final_stats() {
    print_section "Final Statistics"

    local total_users=$(sqlite3 data/lair-chat.db "SELECT COUNT(*) FROM users;" 2>/dev/null || echo "0")
    local admin_users=$(sqlite3 data/lair-chat.db "SELECT COUNT(*) FROM users WHERE role = 'admin';" 2>/dev/null || echo "0")
    local regular_users=$(sqlite3 data/lair-chat.db "SELECT COUNT(*) FROM users WHERE role = 'user';" 2>/dev/null || echo "0")
    local total_messages=$(sqlite3 data/lair-chat.db "SELECT COUNT(*) FROM messages;" 2>/dev/null || echo "0")
    local total_rooms=$(sqlite3 data/lair-chat.db "SELECT COUNT(*) FROM rooms;" 2>/dev/null || echo "0")

    print_info "Database Statistics:"
    echo -e "   • Total Users: ${YELLOW}$total_users${NC}"
    echo -e "   • Admin Users: ${RED}$admin_users${NC}"
    echo -e "   • Regular Users: ${GREEN}$regular_users${NC}"
    echo -e "   • Total Messages: ${BLUE}$total_messages${NC}"
    echo -e "   • Total Rooms: ${CYAN}$total_rooms${NC}"
}

# Main execution function
main() {
    print_banner "🧹 Remove Test Users from Lair Chat"

    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        print_error "Please run this script from the lair-chat directory"
        exit 1
    fi

    # Check for required tools
    if ! command -v sqlite3 &> /dev/null; then
        print_error "sqlite3 is required but not installed"
        exit 1
    fi

    # Stop TCP servers first
    stop_tcp_servers

    # Show current users
    show_all_users

    # Identify test users
    local test_users=$(identify_test_users)

    if [ -n "$test_users" ] && [ "$test_users" != "" ]; then
        echo ""
        echo -e "${YELLOW}Found test users to remove. Continue? (y/N):${NC} "
        read -r confirmation

        if [[ "$confirmation" =~ ^[Yy]$ ]]; then
            remove_test_users "$test_users"
            cleanup_orphaned_data
            vacuum_database
        else
            print_info "Operation cancelled by user"
            exit 0
        fi
    else
        print_info "No test users found to remove"

        echo ""
        echo -e "${YELLOW}Would you like to clean up orphaned data anyway? (y/N):${NC} "
        read -r cleanup_confirmation

        if [[ "$cleanup_confirmation" =~ ^[Yy]$ ]]; then
            cleanup_orphaned_data
            vacuum_database
        fi
    fi

    # Show final statistics
    show_final_stats

    print_banner "✅ Test User Cleanup Complete"
    print_success "All test users have been removed from the system"
    echo ""
    print_info "You can now restart your servers:"
    echo -e "   • Start integrated server: ${BLUE}./quick_start.sh${NC}"
    echo -e "   • Start TCP server only: ${BLUE}cargo run --bin lair-chat-server${NC}"
    echo -e "   • Start REST API only: ${BLUE}cargo run --bin lair-chat-server-new${NC}"
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        echo "Remove Test Users Script for Lair Chat"
        echo ""
        echo "Usage: $0 [options]"
        echo ""
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --force        Remove test users without confirmation"
        echo "  --list-only    Only list test users, don't remove them"
        echo ""
        echo "This script identifies and removes test users from both the"
        echo "TCP server and database, including all associated data."
        echo ""
        echo "Test users are identified by:"
        echo "  • Usernames starting with: test, alice, bob, charlie, demo, guest, user"
        echo "  • Email addresses from test domains: @test.*, @example.*, @localhost"
        echo "  • Non-admin accounts that appear to be for testing"
        exit 0
        ;;
    --force)
        FORCE_REMOVE=true
        ;;
    --list-only)
        LIST_ONLY=true
        ;;
esac

# Run main function if not list-only mode
if [ "${LIST_ONLY:-false}" = "true" ]; then
    print_banner "📋 Test Users List"
    show_all_users
    identify_test_users
else
    main "$@"
fi
