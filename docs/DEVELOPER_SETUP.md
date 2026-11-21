# Developer Setup & Contributor Guide

Welcome to the **Open Business Application Framework (OBAF)** developer setup guide. This document provides step-by-step instructions to get your local development environment configured and running within minutes.

---

## 1. System Prerequisites

OBAF is written in pure asynchronous Rust and is designed with **zero-friction contributor onboarding**. Local execution uses embedded SQLite by default, requiring zero external database daemons or Docker services to start developing.

### Requirements

| Tool | Version | Purpose | Installation |
|---|---|---|---|
| **Rust** | `1.80+` (1.92+ recommended) | Compiler & Cargo package manager | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Rust Components** | `clippy`, `rustfmt` | Code linting and formatting | `rustup component add rustfmt clippy` |
| **Git** | `2.30+` | Source control | Platform package manager |
| **Python** | `3.9+` | Running end-to-end integration test suites | Platform package manager |
| **Node.js** *(Optional)* | `18+` | Building TypeScript Client SDK | `nvm install 20` or official installer |
| **Docker** *(Optional)* | `20+` | Testing containerized production profiles | Docker Desktop or Docker Engine |

---

## ⚡ 1-Click Cloud Workspace (Dev Containers / GitHub Codespaces)

If you don't wish to install Rust, Python, and Node locally, OBAF is fully configured with **VS Code Dev Containers** and **GitHub Codespaces** via [`.devcontainer/devcontainer.json`](../.devcontainer/devcontainer.json):

1. Click **Open in GitHub Codespaces** on GitHub, or press `F1` in VS Code and select `Dev Containers: Reopen in Container`.
2. The pre-configured environment provisions:
   - Rust 1.80+ with `rust-analyzer`, `clippy`, and `rustfmt`
   - Python 3.11 with test tools
   - Node.js 20 for the TypeScript SDK
   - Automatic execution of `make setup` and dependency caching
3. You are immediately ready to run `make tour` or `make dev` without any local host configuration.

---

## 2. 3-Step Quickstart (Local Workstation)

### Step 1: Clone the Repository
```bash
git clone https://github.com/dhirajnavghare/Open-Business-Application-Framework.git
cd Open-Business-Application-Framework
```

### Step 2: Initialize Developer Environment
Run the automated setup target:
```bash
make setup
```
*Creates `data/` directory, copies `.env.example` to `.env`, and verifies Rust toolchain readiness.*

### Step 3: Run the Development Server
```bash
make dev
# Or directly via Cargo:
cargo run -p obaf-server
```

Once running, navigate to:
* 👑 **Super Admin Console:** [http://127.0.0.1:8080/admin](http://127.0.0.1:8080/admin) (Default email: `admin@obaf.local`, password configured via `SUPERADMIN_PASSWORD`)
* 🚀 **Server Health Check:** [http://127.0.0.1:8080/health](http://127.0.0.1:8080/health)
* 📖 **Interactive Swagger UI:** [http://127.0.0.1:8080/swagger-ui](http://127.0.0.1:8080/swagger-ui)
* ⚡ **OpenAPI 3.1 Spec:** [http://127.0.0.1:8080/api/v1/openapi.json](http://127.0.0.1:8080/api/v1/openapi.json)
* 🤖 **AI Agent Context (`llms.txt`):** [http://127.0.0.1:8080/llms.txt](http://127.0.0.1:8080/llms.txt)
* 📊 **Prometheus Metrics:** [http://127.0.0.1:8080/metrics](http://127.0.0.1:8080/metrics)
* 🔍 **Dynamic Schema Discovery:** [http://127.0.0.1:8080/api/v1/meta/schema](http://127.0.0.1:8080/api/v1/meta/schema)

---

## 3. Makefile Developer Reference

A comprehensive [`Makefile`](file:///Users/dhirajnavghare/Developer/Open-Business-Application-Framework/Makefile) is provided at the repository root:

| Command | Action |
|---|---|
| `make setup` | Initialize `.env`, create `data/`, and verify dependencies |
| `make build` | Run all checks and tests, then build release binaries |
| `make dev` / `make run` | Run `obaf-server` in development mode |
| `make status` | Check live server health, response latency, and database metrics (`obaf status`) |
| `make tour` | Run 60-second interactive guided terminal demo (`scripts/tour.sh`) |
| `make worker` | Run the background worker daemon (`obaf-worker`) |
| `make doctor` | Run `obaf doctor` to verify DB, schema, and preset integrity |
| `make presets` | List all 22 domain presets (`obaf presets`) |
| `make check` | Run `cargo fmt --check` and `cargo clippy -- -D warnings` |
| `make fmt` | Automatically format all Rust source code with `cargo fmt` |
| `make test` | Run all 25 Cargo unit and integration test suites |
| `make test-e2e` | Run all Python end-to-end integration test suites |
| `make test-all` | Run all unit tests, SDK checks, and end-to-end suites |
| `make sdk-ts` | Compile and type-check the TypeScript Client SDK |
| `make sdk-py` | Verify and test the Python Client SDK |
| `make install-hooks` | Install Git pre-commit automated test hook |
| `make docker-up` | Start the standalone SQLite Docker Compose profile |
| `make docker-pg` | Start the clustered PostgreSQL + Worker Docker Compose profile |
| `make docker-down` | Stop and tear down all running Docker Compose containers |
| `make clean` | Purge Cargo build cache, dist files, and temporary databases |

---

## 4. Database Setup & Switching

OBAF supports two database profiles with seamless configuration switching:

### Default Profile: Embedded SQLite (Local Development)
Zero external services required. Data is persisted to `data/obaf.db`:
```bash
DATABASE_URL=sqlite://data/obaf.db?mode=rwc
```

### High-Concurrency Profile: PostgreSQL 16+
To develop or benchmark against PostgreSQL locally:
```bash
# 1. Start PostgreSQL 17 container:
docker compose up -d postgres

# 2. Set database connection in .env:
DATABASE_URL=postgres://obaf:obaf_secret@localhost:5432/obaf_db

# 3. Apply schema migrations:
cargo run -p obaf-cli -- migrate

# 4. Start obaf-server:
cargo run -p obaf-server
```

---

## 5. Working with Client SDKs

The repository maintains zero-dependency client SDKs in `sdk/`:

### TypeScript SDK (`sdk/typescript`)
```bash
cd sdk/typescript

# Install dependencies (only TypeScript devDependency)
npm install

# Type-check and build to dist/
npm run typecheck
npm run build
```

### Python SDK (`sdk/python`)
```bash
# Install in editable mode
pip install -e sdk/python

# Run test verification
python tests/test_python_sdk.py
```

---

## 6. Recommended Editor Configuration (VS Code)

To get optimal developer ergonomics in VS Code, install the `rust-analyzer` extension and configure `.vscode/settings.json`:

```json
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": ["--", "-D", "warnings"],
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

---

## 7. Troubleshooting & Common Issues

### Issue: "Database locked" on SQLite during concurrent testing
* **Resolution:** SQLite uses WAL mode (`?mode=rwc`). Ensure connection string includes `mode=rwc` and avoid opening SQLite database files with external graphical tools while running high-concurrency benchmarks.

### Issue: Missing `data/` directory
* **Resolution:** Run `make setup` or `mkdir -p data`. The repository tracks `data/.gitkeep` so fresh clones have this folder created by default.

### Issue: Port 8080 already in use
* **Resolution:** Set custom port via environment variable:
  ```bash
  PORT=9090 cargo run -p obaf-server
  ```

---

## 8. Build-Time Automated Tests & Quality Gates

OBAF enforces automated quality testing across 4 distinct phases of the build and deployment lifecycle:

1. **Cargo Compile-Time Verification (`build.rs`)**:
   - Executes during every `cargo build`.
   - Validates all 22 domain preset JSON schemas in `config/presets/` and validates `config/default.toml`.
   - Any malformed preset or missing configuration aborts compilation immediately.

2. **Docker Multi-Stage Build Verification (`Dockerfile`)**:
   - Executes `cargo test --release --workspace` inside the builder container stage before compilation.
   - Prevents flawed or broken container images from ever being produced or published.

3. **Pre-Build Verification Target (`make build`)**:
   - Invokes `make check` (formatting + clippy) and `make test` (all 25 workspace test suites) prior to building release binaries.

4. **Git Pre-Commit Hook (`.githooks/pre-commit`)**:
   - Automatically runs `cargo fmt --check`, `cargo clippy`, and `cargo test` prior to allowing git commits.
   - Install with: `make install-hooks` (or `git config core.hooksPath .githooks`).

