# Open Business Application Framework (OBAF) Production Deployment & Operations Runbook

This runbook outlines operational procedures, architecture profiles, environment configurations, and disaster recovery policies for running OBAF in production.

---

## 1. Architecture Profiles

OBAF supports two primary operating environments:

| Feature | Development / Edge Profile | Production High-Availability Profile |
|---|---|---|
| **Database** | Embedded SQLite (`data/obaf.db`) | PostgreSQL 16+ Clustered |
| **Connection Pool** | Min: 1, Max: 5 connections | Min: 10, Max: 50 connections with pooling |
| **Worker Threads** | Multi-threaded Tokio (Default) | Multi-core CPU pinning with Tokio runtime |
| **Log Format** | Colorized Pretty Text (`DEBUG`/`INFO`) | Structured JSON (`INFO`/`WARN`) with tracing |
| **Metrics** | `/metrics` endpoint (Prometheus) | Scraped by Prometheus / Grafana Agent every 15s |
| **Storage** | Local volume mount | S3 / Cloud Object Storage / Persistent SSD |

---

## 2. Environment Variables & Configuration

Configuration is loaded hierarchically from `config/default.toml`, environment variables prefixed with `OBAF_`, and `DATABASE_URL`.

### Essential Production Variables
```bash
# Server Port and Host
OBAF_SERVER_PORT=8080
OBAF_SERVER_HOST=0.0.0.0

# Production Database (PostgreSQL)
DATABASE_URL=postgres://obaf_user:strong_password@postgres.internal:5432/obaf_production

# Security & Secrets (Must be 32+ characters)
OBAF_SECURITY_JWT_SECRET=production_ultra_secure_jwt_secret_key_random_32_bytes_min
OBAF_SECURITY_TOKEN_EXPIRY_HOURS=24

# CORS Allowed Origins
OBAF_CORS_ALLOWED_ORIGINS="https://app.example.com,https://pos.example.com"

# Logging Level
RUST_LOG=info,obaf_api=info,obaf_server=info
```

---

## 3. Production PostgreSQL Tuning

For high throughput with SeaORM connection pools, configure PostgreSQL `postgresql.conf`:

```ini
# Memory Configuration (Assuming 8GB RAM host)
shared_buffers = 2GB
effective_cache_size = 6GB
maintenance_work_mem = 512MB
work_mem = 32MB

# Checkpoints and WAL
wal_buffers = 16MB
min_wal_size = 1GB
max_wal_size = 4GB
checkpoint_completion_target = 0.9

# Connection Limits
max_connections = 200
```

---

## 4. Prometheus Monitoring & Alerting

Prometheus scrapes the `/metrics` endpoint over HTTP:

### `prometheus.yml` Configuration:
```yaml
scrape_configs:
  - job_name: 'obaf_nodes'
    scrape_interval: 15s
    static_configs:
      - targets: ['api-node-1:8080', 'api-node-2:8080']
```

### Critical Alerting Rules:
```yaml
groups:
  - name: obaf_alerts
    rules:
      - alert: ObafInstanceDown
        expr: obaf_up == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "OBAF API instance is unreachable"

      - alert: ObafDatabaseDisconnected
        expr: obaf_database_connected == 0
        for: 30s
        labels:
          severity: critical
        annotations:
          summary: "OBAF lost connection to database pool"
```

---

## 5. Zero-Downtime Rolling Upgrades

Because OBAF migrations are strictly additive (Phase 6 compatibility contract), zero-downtime rolling deploys can be conducted:

1. **Step 1: Backup Database State**
   ```bash
   # SQLite
   cargo run -p obaf-cli -- backup --source data/obaf.db
   # PostgreSQL
   pg_dump -Fc -h postgres.internal -U obaf_user obaf_production > obaf_preupgrade.dump
   ```
2. **Step 2: Deploy Migration Canary**
   Start a single canary container with the new binary. On startup, `run_migrations` automatically executes non-blocking `CREATE TABLE IF NOT EXISTS` and additive schema statements.
3. **Step 3: Rolling Pod / Container Update**
   Roll out the new release across remaining instances behind the load balancer with health check polling against `/health`.
4. **Step 4: Verify Health and Metrics**
   Confirm `obaf_up 1` and `obaf_database_connected 1` across all instances.

---

## 6. Disaster Recovery & Rollback

If a critical failure occurs during deployment:
1. Re-route traffic to the previous known good deployment revision.
2. In the event of catastrophic data corruption, restore from the verified backup:
   ```bash
   cargo run -p obaf-cli -- restore --source data/backups/obaf_timestamp.db --destination data/obaf.db
   ```
3. Restart `obaf-server` daemon.
