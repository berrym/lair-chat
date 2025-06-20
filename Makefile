# Lair Chat Development Makefile
# Provides common development tasks and workflows

.PHONY: help setup build test lint clean dev docs docker
.DEFAULT_GOAL := help

# Colors for output
CYAN := \033[0;36m
GREEN := \033[0;32m
YELLOW := \033[1;33m
RED := \033[0;31m
NC := \033[0m # No Color

# Project configuration
PROJECT_NAME := lair-chat
RUST_VERSION := 1.70
DATABASE_URL := postgresql://lair_chat_user:password@localhost/lair_chat_dev
REDIS_URL := redis://localhost:6379

help: ## Show this help message
	@echo "$(CYAN)Lair Chat Development Commands$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Examples:$(NC)"
	@echo "  make setup     # First-time development setup"
	@echo "  make dev       # Start development servers"
	@echo "  make test      # Run all tests"
	@echo "  make check     # Run all checks (lint, test, etc.)"

setup: ## Setup development environment
	@echo "$(CYAN)Setting up development environment...$(NC)"
	@command -v rustc >/dev/null 2>&1 || { echo "$(RED)Error: Rust not found. Please install Rust first.$(NC)"; exit 1; }
	@echo "$(GREEN)✓$(NC) Rust found (version: $$(rustc --version | cut -d' ' -f2))"

	@echo "$(CYAN)Installing Rust components...$(NC)"
	rustup component add clippy rustfmt rust-src

	@echo "$(CYAN)Installing cargo tools...$(NC)"
	cargo install --quiet cargo-watch cargo-audit cargo-deny || true

	@echo "$(CYAN)Setting up environment file...$(NC)"
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo "$(GREEN)✓$(NC) Created .env file from template"; \
	else \
		echo "$(YELLOW)!$(NC) .env file already exists"; \
	fi

	@echo "$(CYAN)Checking database connection...$(NC)"
	@if command -v psql >/dev/null 2>&1; then \
		psql "$(DATABASE_URL)" -c "SELECT 1;" >/dev/null 2>&1 && \
		echo "$(GREEN)✓$(NC) Database connection successful" || \
		echo "$(YELLOW)!$(NC) Database connection failed - please check DATABASE_URL in .env"; \
	else \
		echo "$(YELLOW)!$(NC) PostgreSQL client not found - skipping database check"; \
	fi

	@echo "$(CYAN)Checking Redis connection...$(NC)"
	@if command -v redis-cli >/dev/null 2>&1; then \
		redis-cli -u "$(REDIS_URL)" ping >/dev/null 2>&1 && \
		echo "$(GREEN)✓$(NC) Redis connection successful" || \
		echo "$(YELLOW)!$(NC) Redis connection failed - please check REDIS_URL in .env"; \
	else \
		echo "$(YELLOW)!$(NC) Redis client not found - skipping Redis check"; \
	fi

	@echo "$(CYAN)Building project...$(NC)"
	cargo build

	@echo "$(GREEN)✓ Development environment setup complete!$(NC)"
	@echo "$(YELLOW)Next steps:$(NC)"
	@echo "  1. Configure database settings in .env if needed"
	@echo "  2. Run 'make dev' to start development servers"
	@echo "  3. Run 'make test' to verify everything works"

build: ## Build the project
	@echo "$(CYAN)Building project...$(NC)"
	cargo build

build-release: ## Build optimized release version
	@echo "$(CYAN)Building release version...$(NC)"
	cargo build --release

test: ## Run all tests
	@echo "$(CYAN)Running tests...$(NC)"
	cargo test --verbose

test-unit: ## Run unit tests only
	@echo "$(CYAN)Running unit tests...$(NC)"
	cargo test --lib

test-integration: ## Run integration tests only
	@echo "$(CYAN)Running integration tests...$(NC)"
	cargo test --test '*'

test-watch: ## Run tests in watch mode
	@echo "$(CYAN)Running tests in watch mode...$(NC)"
	cargo watch -x test

test-multi-client: ## Test with multiple clients (requires build)
	@echo "$(CYAN)Running multi-client test...$(NC)"
	./scripts/test_multiple_clients.sh -n 3

test-load: ## Run load test with multiple clients
	@echo "$(CYAN)Running load test...$(NC)"
	./scripts/test_multiple_clients.sh -n 10 -a -d 60

test-quick: ## Quick test setup with server and manual clients
	@echo "$(CYAN)Starting quick test environment...$(NC)"
	./scripts/quick_start.sh

lint: ## Run linting checks
	@echo "$(CYAN)Running linting checks...$(NC)"
	cargo fmt --check
	cargo clippy -- -D warnings

lint-fix: ## Fix linting issues automatically
	@echo "$(CYAN)Fixing linting issues...$(NC)"
	cargo fmt
	cargo clippy --fix --allow-dirty

audit: ## Run security audit
	@echo "$(CYAN)Running security audit...$(NC)"
	cargo audit
	cargo deny check

check: lint audit test ## Run all checks (lint, audit, test)

clean: ## Clean build artifacts
	@echo "$(CYAN)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf target/
	rm -rf coverage/
	rm -f *.log

dev: ## Start development servers
	@echo "$(CYAN)Starting development servers...$(NC)"
	@echo "$(YELLOW)Starting server in background...$(NC)"
	@RUST_LOG=debug cargo run --bin lair-chat-server &
	@echo $$! > .server.pid
	@sleep 2
	@echo "$(GREEN)✓$(NC) Server started (PID: $$(cat .server.pid))"
	@echo "$(YELLOW)Starting client...$(NC)"
	@RUST_LOG=debug cargo run --bin lair-chat-client || true
	@echo "$(CYAN)Stopping server...$(NC)"
	@if [ -f .server.pid ]; then \
		kill $$(cat .server.pid) 2>/dev/null || true; \
		rm -f .server.pid; \
	fi

dev-server: ## Start only the server in development mode
	@echo "$(CYAN)Starting development server...$(NC)"
	RUST_LOG=debug cargo run --bin lair-chat-server

dev-client: ## Start only the client in development mode
	@echo "$(CYAN)Starting development client...$(NC)"
	RUST_LOG=debug cargo run --bin lair-chat-client

watch: ## Run project in watch mode (auto-rebuild on changes)
	@echo "$(CYAN)Starting watch mode...$(NC)"
	cargo watch -x 'run --bin lair-chat-server'

db-setup: ## Setup database (create tables, run migrations)
	@echo "$(CYAN)Setting up database...$(NC)"
	@if command -v sqlx >/dev/null 2>&1; then \
		sqlx database create; \
		sqlx migrate run; \
		echo "$(GREEN)✓$(NC) Database setup complete"; \
	else \
		echo "$(RED)Error: sqlx-cli not found. Install with: cargo install sqlx-cli$(NC)"; \
		exit 1; \
	fi

db-reset: ## Reset database (drop and recreate)
	@echo "$(CYAN)Resetting database...$(NC)"
	@if command -v sqlx >/dev/null 2>&1; then \
		sqlx database drop -y; \
		sqlx database create; \
		sqlx migrate run; \
		echo "$(GREEN)✓$(NC) Database reset complete"; \
	else \
		echo "$(RED)Error: sqlx-cli not found. Install with: cargo install sqlx-cli$(NC)"; \
		exit 1; \
	fi

db-migrate: ## Run database migrations
	@echo "$(CYAN)Running database migrations...$(NC)"
	@if command -v sqlx >/dev/null 2>&1; then \
		sqlx migrate run; \
		echo "$(GREEN)✓$(NC) Migrations complete"; \
	else \
		echo "$(RED)Error: sqlx-cli not found. Install with: cargo install sqlx-cli$(NC)"; \
		exit 1; \
	fi

docs: ## Generate and serve documentation
	@echo "$(CYAN)Generating documentation...$(NC)"
	cargo doc --no-deps --open

docs-serve: ## Serve documentation locally
	@echo "$(CYAN)Serving documentation at http://localhost:8000$(NC)"
	@if command -v python3 >/dev/null 2>&1; then \
		cd target/doc && python3 -m http.server 8000; \
	elif command -v python >/dev/null 2>&1; then \
		cd target/doc && python -m SimpleHTTPServer 8000; \
	else \
		echo "$(RED)Error: Python not found for serving docs$(NC)"; \
		exit 1; \
	fi

benchmark: ## Run benchmarks
	@echo "$(CYAN)Running benchmarks...$(NC)"
	cargo bench

coverage: ## Generate test coverage report
	@echo "$(CYAN)Generating coverage report...$(NC)"
	@if command -v cargo-tarpaulin >/dev/null 2>&1; then \
		cargo tarpaulin --out Html --output-dir coverage/; \
		echo "$(GREEN)✓$(NC) Coverage report generated in coverage/"; \
	else \
		echo "$(RED)Error: cargo-tarpaulin not found. Install with: cargo install cargo-tarpaulin$(NC)"; \
		exit 1; \
	fi

docker-build: ## Build Docker image
	@echo "$(CYAN)Building Docker image...$(NC)"
	docker build -t $(PROJECT_NAME):latest .

docker-run: ## Run Docker container
	@echo "$(CYAN)Running Docker container...$(NC)"
	docker run -p 8080:8080 -p 9090:9090 --name $(PROJECT_NAME) $(PROJECT_NAME):latest

docker-dev: ## Run development environment with Docker Compose
	@echo "$(CYAN)Starting development environment with Docker Compose...$(NC)"
	docker-compose up --build

docker-clean: ## Clean Docker images and containers
	@echo "$(CYAN)Cleaning Docker artifacts...$(NC)"
	docker stop $(PROJECT_NAME) 2>/dev/null || true
	docker rm $(PROJECT_NAME) 2>/dev/null || true
	docker rmi $(PROJECT_NAME):latest 2>/dev/null || true

release: ## Create a release build and package
	@echo "$(CYAN)Creating release build...$(NC)"
	cargo build --release
	@echo "$(CYAN)Running final checks...$(NC)"
	$(MAKE) check
	@echo "$(GREEN)✓$(NC) Release build complete: target/release/"

install: ## Install binaries to system
	@echo "$(CYAN)Installing binaries...$(NC)"
	cargo install --path . --bins

uninstall: ## Uninstall binaries from system
	@echo "$(CYAN)Uninstalling binaries...$(NC)"
	cargo uninstall $(PROJECT_NAME)

deps-update: ## Update dependencies
	@echo "$(CYAN)Updating dependencies...$(NC)"
	cargo update
	@echo "$(CYAN)Checking for outdated dependencies...$(NC)"
	@if command -v cargo-outdated >/dev/null 2>&1; then \
		cargo outdated; \
	else \
		echo "$(YELLOW)Install cargo-outdated for dependency checking: cargo install cargo-outdated$(NC)"; \
	fi

security-check: ## Run comprehensive security checks
	@echo "$(CYAN)Running security checks...$(NC)"
	cargo audit
	@if command -v cargo-deny >/dev/null 2>&1; then \
		cargo deny check; \
	else \
		echo "$(YELLOW)Install cargo-deny for additional security checks: cargo install cargo-deny$(NC)"; \
	fi

profile: ## Profile the application for performance
	@echo "$(CYAN)Profiling application...$(NC)"
	@if command -v cargo-flamegraph >/dev/null 2>&1; then \
		sudo cargo flamegraph --bin lair-chat-server; \
		echo "$(GREEN)✓$(NC) Flamegraph generated: flamegraph.svg"; \
	else \
		echo "$(RED)Error: cargo-flamegraph not found. Install with: cargo install flamegraph$(NC)"; \
		exit 1; \
	fi

stop: ## Stop all running development processes
	@echo "$(CYAN)Stopping development processes...$(NC)"
	@if [ -f .server.pid ]; then \
		kill $$(cat .server.pid) 2>/dev/null || true; \
		rm -f .server.pid; \
		echo "$(GREEN)✓$(NC) Server stopped"; \
	fi
	@pkill -f "lair-chat" 2>/dev/null || true

status: ## Show project status and health
	@echo "$(CYAN)Project Status$(NC)"
	@echo "$(YELLOW)===============$(NC)"
	@echo "Project: $(PROJECT_NAME)"
	@echo "Rust version: $$(rustc --version 2>/dev/null || echo 'Not installed')"
	@echo "Cargo version: $$(cargo --version 2>/dev/null || echo 'Not installed')"
	@echo ""
	@echo "$(YELLOW)Dependencies:$(NC)"
	@command -v psql >/dev/null 2>&1 && echo "$(GREEN)✓$(NC) PostgreSQL client" || echo "$(RED)✗$(NC) PostgreSQL client"
	@command -v redis-cli >/dev/null 2>&1 && echo "$(GREEN)✓$(NC) Redis client" || echo "$(RED)✗$(NC) Redis client"
	@command -v docker >/dev/null 2>&1 && echo "$(GREEN)✓$(NC) Docker" || echo "$(RED)✗$(NC) Docker"
	@echo ""
	@echo "$(YELLOW)Build Status:$(NC)"
	@if [ -d target/debug ]; then echo "$(GREEN)✓$(NC) Debug build exists"; else echo "$(RED)✗$(NC) Debug build missing"; fi
	@if [ -d target/release ]; then echo "$(GREEN)✓$(NC) Release build exists"; else echo "$(RED)✗$(NC) Release build missing"; fi
	@echo ""
	@if [ -f .env ]; then \
		echo "$(YELLOW)Environment:$(NC)"; \
		echo "$(GREEN)✓$(NC) .env file exists"; \
	else \
		echo "$(RED)✗$(NC) .env file missing"; \
	fi

# Special targets for CI/CD
ci-setup: ## Setup for CI environment
	rustup component add clippy rustfmt
	cargo install --quiet cargo-audit || true

ci-test: ## Run tests in CI environment
	cargo test --verbose --all-features

ci-build: ## Build for CI environment
	cargo build --verbose --all-features
	cargo build --release --verbose --all-features
