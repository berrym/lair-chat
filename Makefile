# Lair Chat - Makefile for Development Tasks
# Provides common development, testing, and deployment tasks

.PHONY: help build test clean dev prod install check fmt clippy bench doc docker uat deps

# Default target
.DEFAULT_GOAL := help

# Colors for output
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
NC := \033[0m # No Color

# Configuration
RUST_VERSION := 1.70
CARGO_FLAGS := --release
TEST_FLAGS := --lib --tests --benches

help: ## Show this help message
	@echo "$(CYAN)Lair Chat - Development Makefile$(NC)"
	@echo "=================================="
	@echo ""
	@echo "$(GREEN)Available targets:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-15s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Examples:$(NC)"
	@echo "  make dev          # Start development environment"
	@echo "  make test         # Run all tests"
	@echo "  make uat          # Run UAT tests"
	@echo "  make build        # Build release binaries"

# Development Tasks
dev: ## Start development environment with hot-reload
	@echo "$(GREEN)Starting development environment...$(NC)"
	./scripts/dev.sh

prod: ## Start production environment
	@echo "$(GREEN)Starting production environment...$(NC)"
	./scripts/start.sh

# Build Tasks
build: deps ## Build release binaries
	@echo "$(GREEN)Building release binaries...$(NC)"
	cargo build $(CARGO_FLAGS) --bin lair-chat-server-new --bin lair-chat-server --bin lair-chat-client

build-debug: deps ## Build debug binaries
	@echo "$(GREEN)Building debug binaries...$(NC)"
	cargo build --bin lair-chat-server-new --bin lair-chat-server --bin lair-chat-client

install: build ## Install binaries to ~/.cargo/bin
	@echo "$(GREEN)Installing binaries...$(NC)"
	cargo install --path . --bin lair-chat-server-new --bin lair-chat-server --bin lair-chat-client

# Testing Tasks
test: deps ## Run all tests
	@echo "$(GREEN)Running tests...$(NC)"
	cargo test $(TEST_FLAGS)

test-unit: deps ## Run unit tests only
	@echo "$(GREEN)Running unit tests...$(NC)"
	cargo test --lib

test-integration: deps ## Run integration tests
	@echo "$(GREEN)Running integration tests...$(NC)"
	cargo test --test integration

test-watch: deps ## Run tests in watch mode
	@echo "$(GREEN)Running tests in watch mode...$(NC)"
	cargo watch -x "test $(TEST_FLAGS)"

uat: build ## Run User Acceptance Tests
	@echo "$(GREEN)Running UAT tests...$(NC)"
	./scripts/uat-test.sh

# Code Quality Tasks
check: deps ## Check code compilation without building
	@echo "$(GREEN)Checking code...$(NC)"
	cargo check --all-targets

fmt: ## Format code
	@echo "$(GREEN)Formatting code...$(NC)"
	cargo fmt

fmt-check: ## Check code formatting
	@echo "$(GREEN)Checking code formatting...$(NC)"
	cargo fmt --check

clippy: deps ## Run clippy lints
	@echo "$(GREEN)Running clippy...$(NC)"
	cargo clippy --all-targets -- -D warnings

clippy-fix: deps ## Fix clippy warnings automatically
	@echo "$(GREEN)Fixing clippy warnings...$(NC)"
	cargo clippy --fix --all-targets

audit: ## Run security audit
	@echo "$(GREEN)Running security audit...$(NC)"
	cargo audit

# Performance Tasks
bench: deps ## Run benchmarks
	@echo "$(GREEN)Running benchmarks...$(NC)"
	cargo bench

profile: build ## Profile the application
	@echo "$(GREEN)Profiling application...$(NC)"
	cargo run $(CARGO_FLAGS) --bin lair-chat-server-new &
	sleep 5
	curl http://127.0.0.1:8082/api/v1/health
	pkill lair-chat-server-new

# Documentation Tasks
doc: deps ## Generate documentation
	@echo "$(GREEN)Generating documentation...$(NC)"
	cargo doc --no-deps --open

doc-check: deps ## Check documentation
	@echo "$(GREEN)Checking documentation...$(NC)"
	cargo doc --no-deps

# Docker Tasks
docker-build: ## Build Docker image
	@echo "$(GREEN)Building Docker image...$(NC)"
	docker build -t lair-chat:latest .

docker-run: docker-build ## Run Docker container
	@echo "$(GREEN)Running Docker container...$(NC)"
	docker run -d --name lair-chat -p 8080:8080 -p 8082:8082 lair-chat:latest

docker-stop: ## Stop Docker container
	@echo "$(GREEN)Stopping Docker container...$(NC)"
	docker stop lair-chat || true
	docker rm lair-chat || true

docker-compose-up: ## Start with docker-compose
	@echo "$(GREEN)Starting with docker-compose...$(NC)"
	docker-compose up -d

docker-compose-down: ## Stop docker-compose
	@echo "$(GREEN)Stopping docker-compose...$(NC)"
	docker-compose down

# Database Tasks
db-migrate: build ## Run database migrations
	@echo "$(GREEN)Running database migrations...$(NC)"
	cargo run --bin lair-chat-server-new -- --migrate

db-reset: ## Reset database (WARNING: destructive)
	@echo "$(RED)Resetting database...$(NC)"
	rm -f data/lair_chat.db data/lair_chat_dev.db
	mkdir -p data

db-backup: ## Backup database
	@echo "$(GREEN)Backing up database...$(NC)"
	mkdir -p backups
	cp data/lair_chat.db backups/lair_chat_backup_$$(date +%Y%m%d_%H%M%S).db

# Utility Tasks
clean: ## Clean build artifacts
	@echo "$(GREEN)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf target/
	rm -rf logs/
	rm -rf dev-logs/

clean-all: clean ## Clean everything including data
	@echo "$(RED)Cleaning everything including data...$(NC)"
	rm -rf data/
	rm -rf backups/
	rm -rf test_results/

deps: ## Check and install dependencies
	@echo "$(GREEN)Checking dependencies...$(NC)"
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)Error: Rust/Cargo not found. Install from https://rustup.rs/$(NC)"; exit 1; }
	@rustc --version | grep -q "$(RUST_VERSION)" || echo "$(YELLOW)Warning: Rust $(RUST_VERSION)+ recommended$(NC)"

setup: deps ## Setup development environment
	@echo "$(GREEN)Setting up development environment...$(NC)"
	rustup component add clippy rustfmt
	cargo install cargo-watch cargo-audit
	mkdir -p data logs dev-logs backups
	@echo "$(GREEN)Setup complete! Run 'make dev' to start developing.$(NC)"

# Release Tasks
pre-release: fmt clippy test uat ## Run all pre-release checks
	@echo "$(GREEN)All pre-release checks passed!$(NC)"

release-prep: clean build pre-release ## Prepare for release
	@echo "$(GREEN)Release preparation complete!$(NC)"

# Admin Tasks
admin-user: build ## Create admin user
	@echo "$(GREEN)Creating admin user...$(NC)"
	cargo run --bin create_admin_user

admin-dashboard: ## Open admin dashboard
	@echo "$(GREEN)Opening admin dashboard...$(NC)"
	@command -v xdg-open >/dev/null 2>&1 && xdg-open http://127.0.0.1:8082/admin/ || \
	command -v open >/dev/null 2>&1 && open http://127.0.0.1:8082/admin/ || \
	echo "Open http://127.0.0.1:8082/admin/ in your browser"

# Performance and Load Testing
load-test: build ## Run load tests
	@echo "$(GREEN)Running load tests...$(NC)"
	@test -f scripts/load-test.sh && ./scripts/load-test.sh || echo "$(YELLOW)Load test script not found$(NC)"

stress-test: build ## Run stress tests
	@echo "$(GREEN)Running stress tests...$(NC)"
	@test -f scripts/stress-test.sh && ./scripts/stress-test.sh || echo "$(YELLOW)Stress test script not found$(NC)"

# Monitoring
logs: ## Tail application logs
	@echo "$(GREEN)Tailing logs...$(NC)"
	tail -f logs/*.log dev-logs/*.log 2>/dev/null || echo "$(YELLOW)No log files found$(NC)"

status: ## Check service status
	@echo "$(GREEN)Checking service status...$(NC)"
	@curl -s http://127.0.0.1:8082/api/v1/health | grep -q "ok" && echo "$(GREEN)✅ REST API: Running$(NC)" || echo "$(RED)❌ REST API: Not running$(NC)"
	@nc -z 127.0.0.1 8080 2>/dev/null && echo "$(GREEN)✅ TCP Server: Running$(NC)" || echo "$(RED)❌ TCP Server: Not running$(NC)"

# Development Shortcuts
quick: fmt clippy test ## Quick development check (format, lint, test)

full: clean build test uat ## Full build and test cycle

ci: deps fmt-check clippy test ## CI pipeline checks

# Help for specific tasks
help-docker: ## Show Docker-related help
	@echo "$(CYAN)Docker Commands:$(NC)"
	@echo "  make docker-build      # Build Docker image"
	@echo "  make docker-run        # Run in Docker"
	@echo "  make docker-compose-up # Start with docker-compose"

help-testing: ## Show testing-related help
	@echo "$(CYAN)Testing Commands:$(NC)"
	@echo "  make test             # Run all tests"
	@echo "  make test-unit        # Unit tests only"
	@echo "  make test-integration # Integration tests only"
	@echo "  make uat              # User acceptance tests"
	@echo "  make load-test        # Load testing"

help-dev: ## Show development-related help
	@echo "$(CYAN)Development Commands:$(NC)"
	@echo "  make dev              # Start development environment"
	@echo "  make fmt              # Format code"
	@echo "  make clippy           # Run linter"
	@echo "  make quick            # Quick dev check (fmt + clippy + test)"
