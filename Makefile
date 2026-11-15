.PHONY: help setup dev run status worker doctor presets check fmt test test-e2e test-all tour sdk-ts sdk-py docker-up docker-pg docker-down clean build install-hooks

# Default shell
SHELL := /bin/bash

# Default target
help:
	@echo "======================================================================="
	@echo "  Open Business Application Framework (OBAF) - Developer Commands"
	@echo "======================================================================="
	@echo "  make setup         - Initialize .env, create data/ dir, and check tools"
	@echo "  make build         - Run all checks and tests, then build release binaries"
	@echo "  make dev           - Run obaf-server in local development mode"
	@echo "  make status        - Check live server health, metrics, and database stats"
	@echo "  make tour          - Run 60-second interactive guided terminal demo"
	@echo "  make worker        - Run background worker daemon"
	@echo "  make doctor        - Verify database, migrations, and presets"
	@echo "  make presets       - List all 22 domain preset templates"
	@echo "  make check         - Run formatting and clippy linter checks"
	@echo "  make fmt           - Auto-format all Rust code"
	@echo "  make test          - Run all workspace unit and integration tests"
	@echo "  make test-e2e      - Run Python end-to-end multi-domain test suites"
	@echo "  make test-all      - Run unit tests, SDK checks, and e2e tests"
	@echo "  make sdk-ts        - Build and type-check TypeScript client SDK"
	@echo "  make sdk-py        - Test and verify Python client SDK"
	@echo "  make install-hooks - Install Git pre-commit automated test hook"
	@echo "  make docker-up     - Start standalone SQLite container stack"
	@echo "  make docker-pg     - Start clustered PostgreSQL + Worker stack"
	@echo "  make docker-down   - Stop and tear down all Docker containers"
	@echo "  make clean         - Clean Cargo build cache and temporary test files"
	@echo "======================================================================="

setup:
	@echo "🚀 Setting up OBAF developer environment..."
	@mkdir -p data
	@if [ ! -f .env ]; then \
		echo "Creating .env from .env.example..."; \
		cp .env.example .env; \
	fi
	@cargo --version > /dev/null 2>&1 || (echo "❌ Cargo not found. Please install Rust from https://rustup.rs" && exit 1)
	@echo "✅ Setup complete. Run 'make doctor' or 'make dev' to begin."

build: check test sdk-ts sdk-py
	@echo "📦 Building production release binaries..."
	cargo build --release --workspace
	@echo "🧪 Running post-build release binary smoke tests..."
	./target/release/obaf-cli doctor --database-url "sqlite:///tmp/build_smoke.db?mode=rwc"
	./target/release/obaf-cli presets > /dev/null
	./target/release/obaf-cli token --role admin > /dev/null
	./target/release/obaf-worker --database-url "sqlite:///tmp/build_smoke.db?mode=rwc" --once
	@rm -f /tmp/build_smoke.db*
	@echo "✅ Build completed and all automated build-time tests passed!"

install-hooks:
	@echo "🔧 Installing Git pre-commit verification hook..."
	@git config core.hooksPath .githooks
	@chmod +x .githooks/pre-commit
	@echo "✅ Pre-commit hook installed. Automated tests will run on every git commit."

dev:
	cargo run -p obaf-server

run: dev

status:
	cargo run -p obaf-cli -- status

worker:
	cargo run -p obaf-worker -- --interval-secs 30

doctor:
	cargo run -p obaf-cli -- doctor

presets:
	cargo run -p obaf-cli -- presets

check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

test:
	cargo test --workspace

test-e2e:
	@chmod +x scripts/run_e2e_tests.sh
	./scripts/run_e2e_tests.sh

test-all: test test-e2e sdk-ts sdk-py
	@echo "🎉 All workspace tests, SDK checks, and e2e suites passed!"

benchmark:
	@chmod +x scripts/benchmark.sh
	./scripts/benchmark.sh

tour:
	@chmod +x scripts/tour.sh
	./scripts/tour.sh

sdk-ts:
	@echo "Building and testing TypeScript SDK..."
	npx -y -p typescript tsc -p sdk/typescript/tsconfig.json
	node --test sdk/typescript/test/*.test.mjs
	@echo "Verifying TypeScript Quickstart..."
	npx -y -p typescript tsc -p examples/quickstart_typescript/tsconfig.json --noEmit

sdk-py:
	@echo "Testing Python SDK..."
	PYTHONPATH=sdk/python python3 -m unittest discover -s sdk/python/tests
	python3 tests/test_python_sdk.py

docker-up:
	docker compose up -d obaf-sqlite

docker-pg:
	docker compose --profile postgres --profile worker up -d

docker-down:
	docker compose --profile postgres --profile worker down

clean:
	cargo clean
	rm -rf data/*.db* data/backups/* sdk/typescript/dist
