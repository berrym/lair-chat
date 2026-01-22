#!/bin/bash

# Lair Chat Admin System Setup Script
# This script sets up the complete admin dashboard system with all components

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
ADMIN_USERNAME="${ADMIN_USERNAME:-admin}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-AdminPassword123!}"
ADMIN_EMAIL="${ADMIN_EMAIL:-admin@example.com}"
SERVER_PORT="${SERVER_PORT:-8082}"
DASHBOARD_PORT="${DASHBOARD_PORT:-8083}"

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

# Function to check prerequisites
check_prerequisites() {
    print_section "Checking Prerequisites"

    # Check if cargo is available
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust first."
        echo "Visit: https://rustup.rs/"
        exit 1
    fi
    print_success "Cargo found: $(cargo --version)"

    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        print_error "Cargo.toml not found. Please run this script from the lair-chat directory."
        exit 1
    fi
    print_success "Project directory verified"

    # Check for Python (for serving dashboard)
    if command -v python3 &> /dev/null; then
        print_success "Python3 found: $(python3 --version)"
    elif command -v python &> /dev/null; then
        print_success "Python found: $(python --version)"
    else
        print_warning "Python not found. You'll need to serve the admin dashboard manually."
    fi

    # Check for curl (for API testing)
    if command -v curl &> /dev/null; then
        print_success "curl found: $(curl --version | head -n1)"
    else
        print_warning "curl not found. API testing may be limited."
    fi
}

# Function to create project structure
create_structure() {
    print_section "Creating Project Structure"

    print_step "Creating directories..."
    mkdir -p data logs config admin-dashboard/assets admin-dashboard/js admin-dashboard/css
    print_success "Directories created"

    print_step "Setting up data directory permissions..."
    chmod 755 data logs
    print_success "Permissions set"
}

# Function to setup environment
setup_environment() {
    print_section "Setting Up Environment"

    if [ ! -f ".env" ]; then
        print_step "Creating .env file..."
        cat > .env << EOF
# Database Configuration
DATABASE_URL=sqlite:data/lair_chat.db

# JWT Configuration
JWT_SECRET=lair_chat_jwt_secret_$(openssl rand -hex 32)

# Server Configuration
SERVER_HOST=127.0.0.1
SERVER_PORT=${SERVER_PORT}

# Logging Configuration
RUST_LOG=info,lair_chat=debug,tower_http=debug

# Security Configuration
BCRYPT_COST=12
SESSION_TIMEOUT_HOURS=24
REFRESH_TOKEN_ROTATION=true

# Admin Configuration
ENABLE_ADMIN_API=true
ADMIN_SESSION_TIMEOUT=4

# Rate Limiting
RATE_LIMIT_REQUESTS_PER_MINUTE=60
RATE_LIMIT_BURST=10

# CORS Configuration
CORS_ALLOW_ORIGIN=*
CORS_ALLOW_METHODS=GET,POST,PUT,DELETE,OPTIONS
CORS_ALLOW_HEADERS=Content-Type,Authorization

# File Upload Configuration
MAX_FILE_SIZE=10485760
ALLOWED_FILE_TYPES=txt,md,json,csv

# Feature Flags
ENABLE_WEBSOCKETS=true
ENABLE_FILE_UPLOADS=true
ENABLE_AUDIT_LOGGING=true
ENABLE_METRICS=true
EOF
        print_success ".env file created with secure JWT secret"
    else
        print_info ".env file already exists"
    fi

    # Source the environment file
    if [ -f ".env" ]; then
        export $(cat .env | grep -v '^#' | xargs)
        print_success "Environment variables loaded"
    fi
}

# Function to build the project
build_project() {
    print_section "Building Project"

    print_step "Building release binaries..."
    cargo build --release --bin lair-chat-server-new --bin create_admin_user --bin debug_jwt_auth
    print_success "Project built successfully"

    print_step "Verifying binaries..."
    if [ -f "target/release/lair-chat-server-new" ]; then
        print_success "REST API server binary ready"
    else
        print_error "REST API server binary not found"
        exit 1
    fi

    if [ -f "target/release/create_admin_user" ]; then
        print_success "Admin user creation binary ready"
    else
        print_error "Admin user creation binary not found"
        exit 1
    fi
}

# Function to initialize database
init_database() {
    print_section "Initializing Database"

    print_step "Creating admin user..."
    if cargo run --release --bin create_admin_user -- "$ADMIN_USERNAME" "$ADMIN_PASSWORD" "$ADMIN_EMAIL" 2>/dev/null; then
        print_success "Admin user created successfully"
    else
        print_info "Admin user may already exist or creation handled internally"
    fi

    print_step "Running JWT authentication debug..."
    if cargo run --release --bin debug_jwt_auth > logs/jwt_debug.log 2>&1; then
        print_success "JWT system verified successfully"
    else
        print_warning "JWT debug completed with warnings (check logs/jwt_debug.log)"
    fi
}

# Function to enhance admin dashboard
enhance_dashboard() {
    print_section "Enhancing Admin Dashboard"

    print_step "Adding advanced features to dashboard..."

    # Create enhanced CSS
    cat > admin-dashboard/css/enhanced.css << 'EOF'
/* Enhanced Admin Dashboard Styles */
.metric-card {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    border-radius: 12px;
    padding: 1.5rem;
    color: white;
    text-align: center;
    box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
    transition: transform 0.3s ease;
}

.metric-card:hover {
    transform: translateY(-5px);
}

.real-time-indicator {
    display: inline-block;
    width: 8px;
    height: 8px;
    background: #4CAF50;
    border-radius: 50%;
    animation: pulse 2s infinite;
}

@keyframes pulse {
    0% { opacity: 1; }
    50% { opacity: 0.5; }
    100% { opacity: 1; }
}

.log-viewer {
    background: #1a1a1a;
    color: #00ff00;
    font-family: 'Courier New', monospace;
    padding: 1rem;
    border-radius: 8px;
    max-height: 400px;
    overflow-y: auto;
    border: 1px solid #333;
}

.status-indicator {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
}

.status-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #4CAF50;
}

.status-dot.warning { background: #FF9800; }
.status-dot.error { background: #F44336; }

.action-button {
    background: linear-gradient(45deg, #2196F3, #21CBF3);
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 25px;
    color: white;
    font-weight: bold;
    cursor: pointer;
    transition: all 0.3s ease;
}

.action-button:hover {
    transform: scale(1.05);
    box-shadow: 0 4px 15px rgba(33, 150, 243, 0.4);
}

.sidebar {
    background: linear-gradient(180deg, #2c3e50 0%, #34495e 100%);
    color: white;
    padding: 2rem 1rem;
    border-radius: 0 12px 12px 0;
}

.sidebar-item {
    padding: 0.75rem 1rem;
    margin: 0.5rem 0;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.3s ease;
}

.sidebar-item:hover {
    background: rgba(255, 255, 255, 0.1);
}

.sidebar-item.active {
    background: rgba(255, 255, 255, 0.2);
}
EOF

    # Create enhanced JavaScript
    cat > admin-dashboard/js/enhanced.js << 'EOF'
// Enhanced Admin Dashboard JavaScript
class EnhancedDashboard {
    constructor() {
        this.wsConnection = null;
        this.realTimeEnabled = false;
        this.refreshInterval = null;
        this.init();
    }

    init() {
        this.setupWebSocket();
        this.setupRealTimeUpdates();
        this.setupKeyboardShortcuts();
        this.setupNotifications();
    }

    setupWebSocket() {
        // WebSocket connection for real-time updates
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.hostname}:8082/ws`;

        try {
            this.wsConnection = new WebSocket(wsUrl);

            this.wsConnection.onopen = () => {
                console.log('WebSocket connected');
                this.showNotification('Real-time connection established', 'success');
            };

            this.wsConnection.onmessage = (event) => {
                const data = JSON.parse(event.data);
                this.handleRealtimeUpdate(data);
            };

            this.wsConnection.onclose = () => {
                console.log('WebSocket disconnected');
                this.showNotification('Real-time connection lost', 'warning');
                setTimeout(() => this.setupWebSocket(), 5000);
            };
        } catch (error) {
            console.log('WebSocket not available, using polling');
        }
    }

    setupRealTimeUpdates() {
        this.refreshInterval = setInterval(() => {
            if (document.visibilityState === 'visible') {
                this.refreshDashboardData();
            }
        }, 10000); // Refresh every 10 seconds
    }

    setupKeyboardShortcuts() {
        document.addEventListener('keydown', (e) => {
            if (e.ctrlKey || e.metaKey) {
                switch (e.key) {
                    case 'r':
                        e.preventDefault();
                        this.refreshDashboardData();
                        break;
                    case '1':
                        e.preventDefault();
                        this.switchTab('overview');
                        break;
                    case '2':
                        e.preventDefault();
                        this.switchTab('users');
                        break;
                    case '3':
                        e.preventDefault();
                        this.switchTab('rooms');
                        break;
                    case '4':
                        e.preventDefault();
                        this.switchTab('system');
                        break;
                }
            }
        });
    }

    setupNotifications() {
        if ('Notification' in window && Notification.permission === 'default') {
            Notification.requestPermission();
        }
    }

    showNotification(message, type = 'info') {
        // Create in-page notification
        const notification = document.createElement('div');
        notification.className = `notification notification-${type}`;
        notification.textContent = message;
        notification.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            background: ${type === 'success' ? '#4CAF50' : type === 'warning' ? '#FF9800' : '#2196F3'};
            color: white;
            padding: 1rem 1.5rem;
            border-radius: 8px;
            box-shadow: 0 4px 15px rgba(0,0,0,0.2);
            z-index: 1000;
            animation: slideIn 0.3s ease;
        `;

        document.body.appendChild(notification);

        setTimeout(() => {
            notification.style.animation = 'slideOut 0.3s ease';
            setTimeout(() => document.body.removeChild(notification), 300);
        }, 3000);

        // Browser notification for important alerts
        if ('Notification' in window && Notification.permission === 'granted' && type === 'error') {
            new Notification('Lair Chat Admin Alert', {
                body: message,
                icon: '/favicon.ico'
            });
        }
    }

    refreshDashboardData() {
        console.log('Refreshing dashboard data...');
        if (typeof loadServerStats === 'function') {
            loadServerStats();
        }
    }

    handleRealtimeUpdate(data) {
        switch (data.type) {
            case 'user_count_update':
                this.updateUserCount(data.count);
                break;
            case 'new_message':
                this.handleNewMessage(data);
                break;
            case 'system_alert':
                this.showNotification(data.message, 'warning');
                break;
        }
    }

    updateUserCount(count) {
        const element = document.getElementById('stat-online');
        if (element) {
            element.textContent = count;
        }
    }

    switchTab(tabName) {
        if (typeof switchTab === 'function') {
            switchTab(tabName);
        }
    }
}

// Initialize enhanced dashboard when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.enhancedDashboard = new EnhancedDashboard();
});

// Add CSS animations
const style = document.createElement('style');
style.textContent = `
    @keyframes slideIn {
        from { transform: translateX(100%); opacity: 0; }
        to { transform: translateX(0); opacity: 1; }
    }
    @keyframes slideOut {
        from { transform: translateX(0); opacity: 1; }
        to { transform: translateX(100%); opacity: 0; }
    }
`;
document.head.appendChild(style);
EOF

    print_success "Enhanced dashboard features added"
}

# Function to create monitoring scripts
create_monitoring() {
    print_section "Creating Monitoring Scripts"

    # Server health check script
    cat > scripts/health_check.sh << 'EOF'
#!/bin/bash
# Server health monitoring script

API_BASE="http://127.0.0.1:8082/api/v1"
LOG_FILE="logs/health_check.log"

check_endpoint() {
    local endpoint="$1"
    local name="$2"

    status_code=$(curl -s -o /dev/null -w "%{http_code}" "$API_BASE$endpoint" 2>/dev/null)

    if [ "$status_code" = "200" ]; then
        echo "$(date): $name - OK ($status_code)" >> "$LOG_FILE"
        return 0
    else
        echo "$(date): $name - FAILED ($status_code)" >> "$LOG_FILE"
        return 1
    fi
}

echo "$(date): Starting health check..." >> "$LOG_FILE"

# Check main endpoints
check_endpoint "/health" "Health Check"
check_endpoint "/docs" "API Documentation"

echo "$(date): Health check completed" >> "$LOG_FILE"
EOF

    chmod +x scripts/health_check.sh
    print_success "Health monitoring script created"

    # Log rotation script
    cat > scripts/rotate_logs.sh << 'EOF'
#!/bin/bash
# Log rotation script

LOG_DIR="logs"
MAX_SIZE="10M"
MAX_AGE="7"

find "$LOG_DIR" -name "*.log" -size +$MAX_SIZE -exec gzip {} \;
find "$LOG_DIR" -name "*.log.gz" -mtime +$MAX_AGE -delete

echo "$(date): Log rotation completed" >> "$LOG_DIR/rotation.log"
EOF

    chmod +x scripts/rotate_logs.sh
    print_success "Log rotation script created"
}

# Function to create quick start scripts
create_quick_start() {
    print_section "Creating Quick Start Scripts"

    # Quick start script
    cat > quick_start.sh << 'EOF'
#!/bin/bash
# Quick start script for Lair Chat Admin System

echo "🚀 Starting Lair Chat Admin System..."

# Start server
echo "Starting REST API server..."
cargo run --release --bin lair-chat-server-new > logs/server.log 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Start dashboard server
echo "Starting admin dashboard..."
if command -v python3 &> /dev/null; then
    cd admin-dashboard && python3 -m http.server 8083 > ../logs/dashboard.log 2>&1 &
    DASHBOARD_PID=$!
    cd ..
fi

echo "✅ System started!"
echo "📊 Admin Dashboard: http://127.0.0.1:8083"
echo "🔗 API Endpoint: http://127.0.0.1:8082/api/v1"
echo "📚 API Docs: http://127.0.0.1:8082/swagger-ui"
echo ""
echo "🔐 Admin Credentials:"
echo "   Username: admin"
echo "   Password: AdminPassword123!"
echo ""
echo "Press Ctrl+C to stop all services"

# Cleanup function
cleanup() {
    echo "Stopping services..."
    kill $SERVER_PID 2>/dev/null || true
    kill $DASHBOARD_PID 2>/dev/null || true
    exit 0
}

trap cleanup SIGINT SIGTERM

# Keep running
while true; do
    sleep 1
done
EOF

    chmod +x quick_start.sh
    print_success "Quick start script created"

    # Development script
    cat > dev_start.sh << 'EOF'
#!/bin/bash
# Development mode startup script

export RUST_LOG=debug,lair_chat=trace
export RUST_BACKTRACE=1

echo "🔧 Starting in development mode..."
echo "📋 Logs will be more verbose"

# Run the quick start
./quick_start.sh
EOF

    chmod +x dev_start.sh
    print_success "Development start script created"
}

# Function to run tests
run_tests() {
    print_section "Running System Tests"

    print_step "Starting server for testing..."
    cargo run --release --bin lair-chat-server-new > logs/test_server.log 2>&1 &
    TEST_SERVER_PID=$!

    # Wait for server to start
    sleep 5

    print_step "Running API tests..."
    if [ -f "test_api.sh" ]; then
        ./test_api.sh > logs/api_test.log 2>&1
        print_success "API tests completed (check logs/api_test.log)"
    else
        print_warning "API test script not found"
    fi

    print_step "Stopping test server..."
    kill $TEST_SERVER_PID 2>/dev/null || true
    sleep 2
    print_success "Test server stopped"
}

# Function to display completion summary
show_completion() {
    print_banner "🎉 Lair Chat Admin System Setup Complete!"

    echo -e "${GREEN}✅ System Components Installed:${NC}"
    echo -e "   • REST API Server (lair-chat-server-new)"
    echo -e "   • Admin Dashboard (Enhanced UI)"
    echo -e "   • Database with Admin User"
    echo -e "   • JWT Authentication System"
    echo -e "   • Monitoring & Logging"
    echo -e "   • Quick Start Scripts"

    echo -e "\n${CYAN}🚀 Quick Start Commands:${NC}"
    echo -e "   ${YELLOW}./quick_start.sh${NC}          - Start all services"
    echo -e "   ${YELLOW}./dev_start.sh${NC}            - Start in development mode"
    echo -e "   ${YELLOW}./test_api.sh${NC}             - Test API endpoints"
    echo -e "   ${YELLOW}cargo run --bin debug_jwt_auth${NC} - Debug authentication"

    echo -e "\n${CYAN}🔗 Access URLs:${NC}"
    echo -e "   Admin Dashboard:  ${BLUE}http://127.0.0.1:${DASHBOARD_PORT}${NC}"
    echo -e "   REST API:         ${BLUE}http://127.0.0.1:${SERVER_PORT}/api/v1${NC}"
    echo -e "   API Documentation: ${BLUE}http://127.0.0.1:${SERVER_PORT}/swagger-ui${NC}"

    echo -e "\n${CYAN}🔐 Admin Credentials:${NC}"
    echo -e "   Username: ${GREEN}${ADMIN_USERNAME}${NC}"
    echo -e "   Password: ${GREEN}${ADMIN_PASSWORD}${NC}"
    echo -e "   Email:    ${GREEN}${ADMIN_EMAIL}${NC}"

    echo -e "\n${CYAN}📁 Important Files:${NC}"
    echo -e "   Configuration: ${YELLOW}.env${NC}"
    echo -e "   Database:      ${YELLOW}data/lair_chat.db${NC}"
    echo -e "   Server Logs:   ${YELLOW}logs/server.log${NC}"
    echo -e "   Debug Logs:    ${YELLOW}logs/jwt_debug.log${NC}"

    echo -e "\n${CYAN}🛠️  Advanced Features Available:${NC}"
    echo -e "   • Real-time WebSocket connections"
    echo -e "   • Role-based access control"
    echo -e "   • Audit logging system"
    echo -e "   • File upload capabilities"
    echo -e "   • Rate limiting & security"
    echo -e "   • Health monitoring"
    echo -e "   • Automated log rotation"

    echo -e "\n${PURPLE}🔥 Next Steps:${NC}"
    echo -e "1. Run ${YELLOW}./quick_start.sh${NC} to start the system"
    echo -e "2. Open the admin dashboard in your browser"
    echo -e "3. Login with the admin credentials above"
    echo -e "4. Explore the API documentation"
    echo -e "5. Check the logs for any issues"

    echo -e "\n${GREEN}Happy administering! 🎯${NC}\n"
}

# Main execution flow
main() {
    print_banner "🏗️ Lair Chat Admin System Setup"

    print_info "This script will set up the complete admin dashboard system"
    print_info "Components: REST API, Admin Dashboard, Database, Authentication"

    # Run setup steps
    check_prerequisites
    create_structure
    setup_environment
    build_project
    init_database
    enhance_dashboard
    create_monitoring
    create_quick_start

    # Optional: Run tests
    read -p "$(echo -e ${YELLOW}Run system tests? [y/N]: ${NC})" -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        run_tests
    fi

    show_completion
}

# Error handling
handle_error() {
    print_error "Setup failed at line $1"
    print_info "Check the logs for more details"
    exit 1
}

trap 'handle_error $LINENO' ERR

# Run main function
main "$@"
