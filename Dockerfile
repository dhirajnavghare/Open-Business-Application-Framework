# ========================================================
# Stage 1: Build stage
# ========================================================
FROM rust:1.92-bookworm AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY binaries ./binaries
COPY examples ./examples

# Build release binary
RUN cargo build --release -p obaf-server

# ========================================================
# Stage 2: Minimal runtime image (~25MB)
# ========================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/obaf-server /app/obaf-server

# Copy default config and presets
COPY config /app/config

# Create data directory for SQLite
RUN mkdir -p /app/data

ENV PORT=8080
ENV HOST=0.0.0.0
ENV DATABASE_URL=sqlite:///app/data/obaf.db?mode=rwc

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/app/obaf-server"]
