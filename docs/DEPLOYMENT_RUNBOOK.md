# Open Business Application Framework (OBAF) Production Deployment & Operations Runbook

This runbook provides complete operational guidance, deployment architectures, environment configuration, database tuning, containerization, Kubernetes orchestration, observability, zero-downtime upgrades, and disaster recovery procedures for running OBAF in production.

---

## 1. Architectural Deployment Topologies

OBAF supports two primary operational topologies depending on throughput, latency, and resilience requirements:

### Topology A: Single-Node Edge & Appliance Profile
Designed for edge POS terminals, in-store gateways, pop-up kiosks, or localized branch servers:
* **Database**: Embedded SQLite with WAL (`Write-Ahead Logging`) mode enabled (`sqlite:///app/data/obaf.db?mode=rwc`).
* **Footprint**: Single static Rust binary (~25MB container image, < 35MB baseline RAM).
* **Storage**: Persistent local NVMe or SSD volume mount.
* **Concurrency**: 500–1,500 operations/sec in-memory and write serialization.
* **Process Model**: Single process running `obaf-server` with built-in Tokio asynchronous runtime.

```
                      ┌────────────────────────────────────┐
                      │    Local Edge Appliance / Kiosk    │
                      │                                    │
LAN Clients / POS ───►│ ┌─────────────┐   ┌──────────────┐ │
   HTTP/REST & WS     │ │ obaf-server │──►│ SQLite (WAL) │ │
                      │ └─────────────┘   └──────────────┘ │
                      └────────────────────────────────────┘
```

### Topology B: Clustered High-Availability Cloud Profile
Designed for multi-tenant SaaS, enterprise retail chains, and high-concurrency billing platforms:
* **Stateless API Tier**: Horizontally scaled `obaf-server` replicas running behind an L7 Load Balancer (AWS ALB, GCP Cloud Load Balancer, Cloudflare, or NGINX/Envoy).
* **Database Tier**: Clustered PostgreSQL 16+ (Primary for writes, optional Read Replicas, Multi-AZ automated failover).
* **Tenant Isolation**: Shared schema with logical tenant isolation by default, with dynamic switching to dedicated database pools via `TenantPoolRouter` for enterprise tiers.
* **Background Worker Tier**: Dedicated `obaf-worker` daemons running periodic maintenance sweeps and webhook dispatch reconciliations.
* **Observability**: Prometheus scraping `/metrics` and `/health`, distributed tracing via `x-request-id`.

```
                         ┌─────────────────────────────────┐
                         │   Global L7 Load Balancer / CDN │
                         │   (TLS 1.3 / WAF / HTTP/2 & WS) │
                         └────────────────┬────────────────┘
                                          │
                 ┌────────────────────────┴────────────────────────┐
                 ▼                                                 ▼
      ┌─────────────────────┐                           ┌─────────────────────┐
      │  obaf-server Pod 1  │                           │  obaf-server Pod N  │
      │  (Stateless Rust)   │                           │  (Stateless Rust)   │
      └──────────┬──────────┘                           └──────────┬──────────┘
                 │                                                 │
                 ├────────────────────────┬────────────────────────┤
                 ▼                        ▼                        ▼
      ┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────┐
      │ PostgreSQL Primary  │  │ PostgreSQL Replica  │  │ obaf-worker Daemon  │
      │ (Writes & Reads)    │  │ (Read-heavy reports)│  │ (Background sweeps) │
      └─────────────────────┘  └─────────────────────┘  └─────────────────────┘
```

---

## 2. Environment Variables & Configuration Reference

OBAF evaluates configuration using a deterministic priority cascade:
1. Direct Environment Overrides (`DATABASE_URL`, `PORT`, `HOST`, `JWT_SECRET`)
2. Double-Underscore Namespaced Overrides (`OBAF_SERVER__PORT`, `OBAF_SECURITY__JWT_SECRET`)
3. Flat Namespaced Overrides (`OBAF_SERVER_PORT`, `OBAF_SECURITY_JWT_SECRET`, `OBAF_WORKER_INTERVAL_SECS`)
4. File Configuration (`config/default.toml` or file designated by `OBAF_CONFIG_PATH`)
5. Hardcoded Defaults

### Comprehensive Configuration Matrix

| Variable | Config Key | Type | Default | Description |
|---|---|---|---|---|
| `OBAF_SERVER_PORT` / `PORT` / `OBAF_SERVER__PORT` | `server.port` | `u16` | `8080` | TCP port the HTTP and WebSocket listener binds to |
| `OBAF_SERVER_HOST` / `HOST` / `OBAF_SERVER__HOST` | `server.host` | `string` | `127.0.0.1` (`0.0.0.0` in container) | Network interface IP address to bind |
| `DATABASE_URL` / `OBAF_DATABASE_URL` | `database.url` | `string` | `sqlite://data/obaf.db?mode=rwc` | PostgreSQL or SQLite primary connection URI |
| `DATABASE_REPLICAS` / `OBAF_DATABASE_REPLICAS` | `database.replicas` | `list<string>` | `[]` | Comma-separated list of read-only replica database URIs for automatic round-robin query load balancing |
| `DATABASE_MAX_CONNECTIONS` / `OBAF_DATABASE_MAX_CONNECTIONS` | N/A | `u32` | `50` (PG) / `25` (SQLite) | Maximum concurrent connections in the connection pool |
| `DATABASE_MIN_CONNECTIONS` / `OBAF_DATABASE_MIN_CONNECTIONS` | N/A | `u32` | `5` (PG) / `1` (SQLite) | Minimum idle connections preserved in the pool |
| `OBAF_SECURITY_JWT_SECRET` / `JWT_SECRET` | `security.jwt_secret` | `string` | `dev_secret...` | High-entropy secret key for HMAC-SHA256 JWT signing (≥ 32 chars in production) |
| `OBAF_SECURITY_TOKEN_EXPIRY_HOURS` | `security.token_expiry_hours` | `u64` | `24` | Default validity window in hours for generated auth tokens |
| `OBAF_WORKER_INTERVAL_SECS` | N/A | `u64` | `30` | Interval in seconds between reconciliation sweeps in `obaf-worker` |
| `OBAF_CONFIG_PATH` | N/A | `string` | `config/default.toml` | Explicit filesystem path to TOML configuration file |
| `RUST_LOG` | N/A | `string` | `info` | Tracing log filter expression (e.g. `info,obaf_api=debug`) |

---

## 3. Database Architecture & Production PostgreSQL Sizing

### 3.1 Connection Pool Mathematics

OBAF utilizes `sea-orm` (backed by `sqlx`) with an asynchronous connection pool. When sizing PostgreSQL `max_connections`, apply this formula:

$$\text{Postgres max\_connections} \ge (\text{API Replicas} \times \text{Pool Max Connections}) + (\text{Worker Replicas} \times \text{Worker Pool}) + \text{Admin Reserve (15)}$$

* Default OBAF API pool limits: `min_connections = 5`, `max_connections = 50`.
* For a 3-replica API deployment: $(3 \times 50) + (1 \times 10) + 15 = 175$ connections minimum.

### 3.2 Production `postgresql.conf` Tuning Profiles

Configure PostgreSQL 16+ according to dedicated instance RAM:

```ini
# ==============================================================================
# OBAF PostgreSQL 16+ Recommended Production Tuning (16GB RAM Dedicated Host)
# ==============================================================================

# Memory Allocation
shared_buffers = 4GB                  # 25% of total system RAM
effective_cache_size = 12GB           # 75% of total system RAM
maintenance_work_mem = 1GB            # Maintenance operations (indices, VACUUM)
work_mem = 64MB                       # Sized for complex ledger aggregations

# WAL & Checkpoint Optimization
wal_buffers = 16MB
min_wal_size = 2GB
max_wal_size = 8GB
checkpoint_completion_target = 0.9    # Spread write spikes over time
checkpoint_timeout = 15min

# Concurrency & Worker Processes
max_connections = 250
max_worker_processes = 8
max_parallel_workers_per_gather = 4
max_parallel_maintenance_workers = 4

# Query Planner & Disk Architecture (Assuming NVMe/SSD)
random_page_cost = 1.1                # Fast SSD random reads
effective_io_concurrency = 200

# Automated Background Vacuuming
autovacuum = on
autovacuum_max_workers = 4
autovacuum_naptime = 1min
autovacuum_vacuum_scale_factor = 0.05
autovacuum_analyze_scale_factor = 0.02
```

---

## 4. Containerization & Docker Deployment

### 4.1 Production Multi-Stage `Dockerfile`

The repository includes a multi-stage `Dockerfile` producing an ultra-lightweight (~25MB), unprivileged runtime image:

```dockerfile
# Stage 1: Cargo Release Builder
FROM rust:1.92-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY binaries ./binaries
COPY examples ./examples
RUN cargo build --release -p obaf-server -p obaf-cli -p obaf-worker

# Stage 2: Distroless-like Minimal Debian Runtime
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
RUN groupadd -r obaf && useradd -r -g obaf -d /app -s /sbin/nologin obaf
COPY --from=builder /app/target/release/obaf-server /app/obaf-server
COPY --from=builder /app/target/release/obaf-cli /app/obaf-cli
COPY --from=builder /app/target/release/obaf-worker /app/obaf-worker
COPY config /app/config
RUN mkdir -p /app/data && chown -R obaf:obaf /app
USER obaf
ENV PORT=8080 HOST=0.0.0.0 DATABASE_URL=sqlite:///app/data/obaf.db?mode=rwc
EXPOSE 8080
HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://127.0.0.1:8080/health || exit 1
ENTRYPOINT ["/app/obaf-server"]
```

### 4.2 Docker Compose Profiles

The root [`docker-compose.yml`](file:///Users/dhirajnavghare/Developer/Open-Business-Application-Framework/docker-compose.yml) provides turnkey operational profiles:

```bash
# 1. Start Zero-Dependency SQLite Stack (Ideal for local testing, pop-up kiosks, edge nodes):
docker compose up -d obaf-sqlite

# 2. Start Clustered PostgreSQL Stack:
docker compose --profile postgres up -d

# 3. Start PostgreSQL Stack with Background Worker:
docker compose --profile postgres --profile worker up -d

# 4. Turnkey Single-Server Production Deployment (Automated Let's Encrypt TLS via Caddy):
DOMAIN=api.mybusiness.com docker compose --profile production up -d

# 5. View real-time aggregated logs:
docker compose logs -f
```

### 4.3 Bare-Metal / Virtual Machine Systemd Deployment (Non-Docker)

For bare-metal Linux servers (Debian, Ubuntu, RHEL, Arch) without container overhead, hardened systemd service units are provided in `scripts/systemd/`:

```bash
# 1. Install compiled binaries
sudo cp target/release/obaf-server /usr/local/bin/
sudo cp target/release/obaf-worker /usr/local/bin/
sudo cp target/release/obaf-cli /usr/local/bin/

# 2. Create unprivileged system user & working directories
sudo useradd -r -s /bin/false obaf
sudo mkdir -p /var/lib/obaf /etc/obaf
sudo chown -R obaf:obaf /var/lib/obaf

# 3. Configure production environment file
sudo tee /etc/obaf/obaf.env > /dev/null << 'EOF'
PORT=8080
HOST=127.0.0.1
DATABASE_URL=postgres://obaf:secure_password@127.0.0.1:5432/obaf_prod
JWT_SECRET=production_high_entropy_jwt_secret_min_32_characters
RUST_LOG=info
EOF
sudo chmod 600 /etc/obaf/obaf.env

# 4. Install and start systemd units
sudo cp scripts/systemd/obaf-server.service /etc/systemd/system/
sudo cp scripts/systemd/obaf-worker.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now obaf-server obaf-worker
```

### 4.4 OS-Independence & Multi-Architecture Support

Deployment via Docker is **100% OS-independent**. Because OBAF's application binaries, runtime libraries (glibc/musl, OpenSSL, CA certificates), timezone databases, and configurations are encapsulated inside the OCI-compliant container image (`debian:bookworm-slim`), the host operating system is completely decoupled from the runtime execution:

| Host Operating System | Container Engine | Runtime Layer | Supported Architectures |
|---|---|---|---|
| **Linux** (Ubuntu, Debian, RHEL, Rocky, Alpine, Arch, Amazon Linux) | Docker Engine / Podman / containerd | Native Linux Kernel cgroups & namespaces | `linux/amd64`, `linux/arm64` |
| **macOS** (Apple Silicon M1/M2/M3/M4 & Intel Core) | Docker Desktop / OrbStack / Colima | Lightweight Virtualized Linux Hypervisor | `linux/arm64` (Native), `linux/amd64` (Rosetta 2) |
| **Windows** (Windows 10 / 11 / Windows Server) | Docker Desktop / Rancher Desktop | WSL2 (Windows Subsystem for Linux 2) | `linux/amd64`, `linux/arm64` |
| **Cloud Managed Containers** (AWS ECS/Fargate, GCP Cloud Run, Azure Container Apps, DigitalOcean) | Managed Container Runtime | Firecracker microVM / OCI Container Engine | `linux/amd64`, `linux/arm64` |

#### Multi-Architecture Production Builds with Docker Buildx:
To build and push multi-architecture images targeting both standard x86_64 servers (AWS EC2, GCP Compute Engine, Intel/AMD data centers) and ARM64 servers (AWS Graviton 3/4, Ampere Altra, Apple Silicon, Raspberry Pi edge devices):

```bash
# 1. Create and initialize a multi-platform buildx builder instance:
docker buildx create --name obaf-builder --use
docker buildx inspect --bootstrap

# 2. Build and publish multi-arch images simultaneously:
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ghcr.io/your-org/obaf:latest \
  -t ghcr.io/your-org/obaf:0.1.0 \
  --push .
```

#### Cross-Platform Volume Mounting Considerations:
* **Linux**: Local bind mounts (`./data:/app/data`) inherit the host system's file permissions. Named volumes (`obaf-data:/app/data`) are recommended for multi-tenant production to prevent UID/GID mismatches.
* **macOS & Windows**: Docker Desktop automatically handles file path translation (`/` vs `\`) and UID translation between the host filesystem and the container's unprivileged `obaf` user (UID 999). No host changes are necessary.


---

## 5. Kubernetes (K8s) Production Orchestration

For cloud-native deployments (EKS, GKE, AKS, or bare-metal k8s), deploy the following production-grade manifests:

### 5.1 Namespace, Secrets, and ConfigMap (`k8s-config.yaml`)

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: obaf-production
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: obaf-server-config
  namespace: obaf-production
data:
  OBAF_SERVER_HOST: "0.0.0.0"
  OBAF_SERVER_PORT: "8080"
  RUST_LOG: "info,obaf_api=info,obaf_server=info,obaf_database=info"
  OBAF_WORKER_INTERVAL_SECS: "30"
---
apiVersion: v1
kind: Secret
metadata:
  name: obaf-secrets
  namespace: obaf-production
type: Opaque
stringData:
  DATABASE_URL: "postgres://obaf_user:REPLACE_WITH_STRONG_PASSWORD@postgres.internal:5432/obaf_production"
  JWT_SECRET: "REPLACE_WITH_HIGH_ENTROPY_RANDOM_SECRET_KEY_MIN_32_BYTES"
```

### 5.2 Stateless API Deployment (`k8s-api-deployment.yaml`)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: obaf-api
  namespace: obaf-production
  labels:
    app.kubernetes.io/name: obaf-api
    app.kubernetes.io/part-of: obaf
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app.kubernetes.io/name: obaf-api
  template:
    metadata:
      labels:
        app.kubernetes.io/name: obaf-api
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
        prometheus.io/path: "/metrics"
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 999
        fsGroup: 999
      containers:
        - name: obaf-server
          image: ghcr.io/your-org/obaf:latest
          imagePullPolicy: IfNotPresent
          ports:
            - name: http
              containerPort: 8080
              protocol: TCP
          envFrom:
            - configMapRef:
                name: obaf-server-config
            - secretRef:
                name: obaf-secrets
          resources:
            requests:
              cpu: "100m"
              memory: "64Mi"
            limits:
              cpu: "1000m"
              memory: "256Mi"
          livenessProbe:
            httpGet:
              path: /live
              port: http
            initialDelaySeconds: 5
            periodSeconds: 10
            timeoutSeconds: 2
            failureThreshold: 3
          readinessProbe:
            httpGet:
              path: /ready
              port: http
            initialDelaySeconds: 3
            periodSeconds: 5
            timeoutSeconds: 2
            successThreshold: 1
            failureThreshold: 2
---
apiVersion: v1
kind: Service
metadata:
  name: obaf-api-service
  namespace: obaf-production
spec:
  type: ClusterIP
  ports:
    - name: http
      port: 80
      targetPort: http
  selector:
    app.kubernetes.io/name: obaf-api
```

### 5.3 Horizontal Pod Autoscaler (HPA) & Disruption Budget

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: obaf-api-hpa
  namespace: obaf-production
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: obaf-api
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 75
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
---
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: obaf-api-pdb
  namespace: obaf-production
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app.kubernetes.io/name: obaf-api
```

### 5.4 Dedicated Background Worker Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: obaf-worker
  namespace: obaf-production
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: obaf-worker
  template:
    metadata:
      labels:
        app.kubernetes.io/name: obaf-worker
    spec:
      containers:
        - name: obaf-worker
          image: ghcr.io/your-org/obaf:latest
          command: ["/app/obaf-worker"]
          args: ["--interval-secs", "30"]
          envFrom:
            - configMapRef:
                name: obaf-server-config
            - secretRef:
                name: obaf-secrets
          resources:
            requests:
              cpu: "50m"
              memory: "32Mi"
            limits:
              cpu: "500m"
              memory: "128Mi"
```

---

## 6. Reverse Proxy & Ingress Hardening (NGINX Example)

When deploying behind NGINX or an Ingress Controller, ensure WebSocket upgrades, real client IP headers, and HTTP/2 are configured:

```nginx
upstream obaf_upstream {
    server 10.0.1.10:8080 max_fails=3 fail_timeout=10s;
    server 10.0.1.11:8080 max_fails=3 fail_timeout=10s;
    keepalive 64;
}

server {
    listen 443 ssl http2;
    server_name api.business.example.com;

    # SSL / TLS 1.3 Configuration
    ssl_certificate /etc/letsencrypt/live/api.business.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.business.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    # Rate Limiting: 100 req/sec per client IP with burst allowance of 30
    limit_req zone=api_ip burst=30 nodelay;

    # Proxy Pass to OBAF Cluster
    location / {
        proxy_pass http://obaf_upstream;
        proxy_http_version 1.1;

        # WebSocket Upgrade Headers (Required for /api/v1/ws)
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Request Correlation & Client Identity
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Request-Id $request_id;

        # Timeouts
        proxy_connect_timeout 5s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
}
```

---

## 7. Observability, Metrics & Production Alerting

### 7.1 Prometheus Metrics Endpoint (`/metrics`)

OBAF natively exposes Prometheus-compatible metrics at `/metrics` over standard HTTP without external daemons:

```text
# HELP obaf_up Service status (1 = up)
# TYPE obaf_up gauge
obaf_up 1
# HELP obaf_database_connected Database connection health (1 = connected, 0 = unreachable)
# TYPE obaf_database_connected gauge
obaf_database_connected 1
# HELP obaf_info Framework build info
# TYPE obaf_info gauge
obaf_info{version="0.1.0"} 1
```

### 7.2 Prometheus Scrape Job (`prometheus.yml`)

```yaml
scrape_configs:
  - job_name: 'obaf_cluster'
    scrape_interval: 15s
    scrape_timeout: 5s
    metrics_path: '/metrics'
    static_configs:
      - targets:
          - 'api-1.internal:8080'
          - 'api-2.internal:8080'
          - 'api-3.internal:8080'
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
```

### 7.3 Prometheus Alertmanager Rules (`alerts.yml`)

```yaml
groups:
  - name: obaf_production_alerts
    rules:
      - alert: ObafInstanceDown
        expr: obaf_up == 0
        for: 1m
        labels:
          severity: critical
          tier: api
        annotations:
          summary: "OBAF API instance {{ $labels.instance }} is unreachable"
          description: "Service has been down for more than 1 minute."

      - alert: ObafDatabaseDisconnected
        expr: obaf_database_connected == 0
        for: 30s
        labels:
          severity: critical
          tier: storage
        annotations:
          summary: "OBAF lost connection to database pool on {{ $labels.instance }}"
          description: "Health ping to primary database connection pool failed."

      - alert: HighRequestFailureRate
        expr: (sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))) * 100 > 2
        for: 2m
        labels:
          severity: warning
          tier: api
        annotations:
          summary: "HTTP 5xx error rate exceeds 2% over 5m window"
```

---

## 8. Zero-Downtime Rolling Upgrades

Because all OBAF schema migrations adhere strictly to the Phase 6 Non-Breaking Schema Compatibility Contract (`CREATE TABLE IF NOT EXISTS`, nullable columns, idempotent default values), zero-downtime rolling upgrades are guaranteed.

### Step-by-Step Production Upgrade Procedure:

1. **Pre-flight Diagnostic Check**:
   Verify current cluster health, schema status, and preset integrity using `obaf doctor`:
   ```bash
   cargo run -p obaf-cli -- doctor --database-url "$DATABASE_URL"
   ```

2. **Execute Database Backup Snapshot**:
   Take a state snapshot prior to executing the rollout:
   ```bash
   # For PostgreSQL:
   pg_dump -Fc -h postgres.internal -U obaf_user obaf_production > "obaf_preupgrade_$(date +%Y%m%d_%H%M%S).dump"

   # For SQLite edge nodes:
   cargo run -p obaf-cli -- backup --source data/obaf.db
   ```

3. **Apply Schema Migrations via Init Job or CLI**:
   Run schema migrations ahead of application binary rollouts:
   ```bash
   cargo run -p obaf-cli -- migrate --database-url "$DATABASE_URL"
   ```

4. **Deploy Single Canary Replica**:
   Deploy the new binary image to 1 canary pod. Verify:
   * `/health` returns `{"status":"ok","database":"connected"}`
   * `/metrics` reports `obaf_up 1` and `obaf_database_connected 1`
   * Monitor error rates in tracing logs (`x-request-id`).

5. **Execute Rolling Update Across Fleet**:
   In Kubernetes or Docker Swarm, proceed with rolling update (`maxUnavailable: 0`, `maxSurge: 1`). Old pods cleanly drain connections via Tokio's graceful `SIGTERM` handler.

---

## 9. Disaster Recovery & Rollback Playbooks

### Playbook A: Immediate Application Rollback (No Schema Change)
If an application defect is discovered in the new release:
1. Re-route load balancer traffic or update Kubernetes image tag back to the previous stable release.
2. Because schema changes are backward-compatible, older versions will continue running seamlessly against the upgraded database.

### Playbook B: PostgreSQL Point-in-Time Database Restoration (Catastrophic Data Loss)
1. Terminate all active API and Worker pods to prevent write skew:
   ```bash
   kubectl scale deployment obaf-api obaf-worker --replicas=0 -n obaf-production
   ```
2. Restore database from pre-upgrade dump:
   ```bash
   pg_restore --clean --if-exists -h postgres.internal -U obaf_user -d obaf_production obaf_preupgrade_*.dump
   ```
3. Verify database tables:
   ```bash
   cargo run -p obaf-cli -- doctor --database-url "$DATABASE_URL"
   ```
4. Restore API and Worker pods to normal replica count:
   ```bash
   kubectl scale deployment obaf-api --replicas=3 -n obaf-production
   kubectl scale deployment obaf-worker --replicas=1 -n obaf-production
   ```

### Playbook C: SQLite Edge Node Database Restoration
1. Stop edge `obaf-server` service:
   ```bash
   systemctl stop obaf-server
   ```
2. Restore the database file using `obaf restore`:
   ```bash
   cargo run -p obaf-cli -- restore --source data/backups/obaf_backup.db --destination data/obaf.db
   ```
3. Restart `obaf-server`:
   ```bash
   systemctl start obaf-server
   ```
4. Confirm health:
   ```bash
   curl -f http://127.0.0.1:8080/health
   ```
