# Multi-stage build for optimized production image
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy dependency files
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN mkdir -p src/bin && echo "fn main() {}" > src/bin/cli.rs && echo "fn main() {}" > src/bin/server.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY src ./src
COPY benches ./benches

# Build the application
RUN touch src/main.rs src/bin/cli.rs src/bin/server.rs
RUN cargo build --release

# Production stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false -m -d /app indexer

# Set working directory
WORKDIR /app

# Copy binaries from builder stage
COPY --from=builder /app/target/release/indexer /usr/local/bin/
COPY --from=builder /app/target/release/cli /usr/local/bin/
COPY --from=builder /app/target/release/server /usr/local/bin/

# Copy configuration files
COPY config.example.toml ./config.toml
COPY scripts/docker-entrypoint.sh ./entrypoint.sh

# Make entrypoint executable
RUN chmod +x ./entrypoint.sh

# Create data directory for database
RUN mkdir -p /app/data && chown -R indexer:indexer /app

# Switch to app user
USER indexer

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/status || exit 1

# Set entrypoint
ENTRYPOINT ["./entrypoint.sh"]

# Default command
CMD ["indexer"]