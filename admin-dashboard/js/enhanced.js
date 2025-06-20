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
