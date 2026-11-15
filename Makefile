.PHONY: help setup dev run worker doctor presets check fmt test test-e2e test-all sdk-ts sdk-py docker-up docker-pg docker-down clean

# Default shell
SHELL := /bin/bash

# Default target
help:
	@echo "======================================================================="
	@echo "  Open Business Application Framework (OBAF) - Developer Commands"
	@echo "======================================================================="
	@echo "  make setup         - Initialize .env, create data/ dir, and check tools"
	@echo "  make dev           - Run obaf-server in local development mode"
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

dev:
	cargo run -p obaf-server

run: dev

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
	@echo "Running Python End-to-End Test Suites..."
	python3 tests/test_flow.py
	python3 tests/test_pharmacy.py
	python3 tests/test_retail_and_hotel.py
	python3 tests/test_services_gym_bookings.py
	python3 tests/test_webhooks_and_invoices.py
	python3 tests/test_procurement_and_promotions_e2e.py

test-all: test test-e2e sdk-ts sdk-py
	@echo "🎉 All workspace tests, SDK checks, and e2e suites passed!"

sdk-ts:
	@echo "Building TypeScript SDK..."
	npx -y -p typescript tsc -p sdk/typescript/tsconfig.json --noEmit

sdk-py:
	@echo "Testing Python SDK..."
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
