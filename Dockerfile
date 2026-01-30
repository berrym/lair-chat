# Build stage
FROM rust:1.88 AS builder

WORKDIR /app

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the server in release mode
RUN cargo build --release --package lair-chat-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user (don't run as root)
RUN useradd --create-home --user-group lair

# Create data directory
RUN mkdir -p /data && chown lair:lair /data

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/lair-chat-server .

# Switch to non-root user
USER lair

# Expose ports
EXPOSE 8080 8443

# Set default environment variables
ENV LAIR_DATABASE_URL=sqlite:/data/lair-chat.db?mode=rwc

# Run the server
CMD ["./lair-chat-server"]


