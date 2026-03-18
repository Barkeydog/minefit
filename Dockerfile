# Multi-stage build for minefit
# Stage 1: Build the Rust binary
FROM rust:1.88-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members
COPY llmfit-core/ ./llmfit-core/
COPY llmfit-tui/ ./llmfit-tui/
COPY llmfit-desktop/ ./llmfit-desktop/
COPY data/ ./data/

# Build release binary for minefit TUI/CLI
RUN cargo build --release -p minefit

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies for hardware detection
RUN apt-get update && apt-get install -y \
    pciutils \
    lshw \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /build/target/release/minefit /usr/local/bin/minefit

# Create a non-root user
RUN useradd -m -u 1000 minefit && \
    chown -R minefit:minefit /usr/local/bin/minefit

USER minefit

# Default to JSON output so the container is useful in automation.
ENTRYPOINT ["/usr/local/bin/minefit"]
CMD ["--json", "-n", "25"]
