# ========================================================
# Stage 1: Build stage
# ========================================================
FROM rust:1.92-bookworm AS builder

WORKDIR /app

# Copy manifests and source code
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY binaries ./binaries
COPY examples ./examples
COPY config ./config
COPY scripts ./scripts
COPY llms.txt README.md LICENSE NOTICE ./

# Execute automated tests during container build to guarantee zero-defect images
RUN cargo test --release --workspace

# Build release binaries
RUN cargo build --release -p obaf-server -p obaf-cli -p obaf-worker

# Run automated release binary smoke tests to verify runtime linking, migration integrity & presets
RUN /app/target/release/obaf-cli doctor --database-url "sqlite:///tmp/build_smoke.db?mode=rwc" \
    && /app/target/release/obaf-cli presets \
    && /app/target/release/obaf-cli token --role admin > /dev/null \
    && /app/target/release/obaf-worker --database-url "sqlite:///tmp/build_smoke.db?mode=rwc" --once \
    && rm -f /tmp/build_smoke.db*

# ========================================================
# Stage 2: Minimal runtime image (~25MB)
# ========================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    gosu \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create unprivileged application user
RUN groupadd -r obaf && useradd -r -g obaf -d /app -s /sbin/nologin obaf

# Copy binaries from builder
COPY --from=builder /app/target/release/obaf-server /app/obaf-server
COPY --from=builder /app/target/release/obaf-cli /app/obaf-cli
COPY --from=builder /app/target/release/obaf-worker /app/obaf-worker

# Copy default config and presets
COPY config /app/config

# Copy license, attribution, and AI documentation
COPY --from=builder /app/LICENSE /app/LICENSE
COPY --from=builder /app/NOTICE /app/NOTICE
COPY --from=builder /app/llms.txt /app/llms.txt

# Copy entrypoint script
COPY scripts/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Create data directory for SQLite and ensure ownership
RUN mkdir -p /app/data && chown -R obaf:obaf /app

ENV PORT=8080
ENV HOST=0.0.0.0
ENV DATABASE_URL=sqlite:///app/data/obaf.db?mode=rwc

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["/app/obaf-server"]
