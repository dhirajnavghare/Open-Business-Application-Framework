#!/usr/bin/env bash
# ==============================================================================
# Automated End-to-End Multi-Domain Integration Test Runner
# Spawns an ephemeral OBAF server on a temporary SQLite DB if not already running.
# ==============================================================================
set -e

PORT=8080
BASE_URL="http://127.0.0.1:${PORT}"
TMP_DB="/tmp/obaf_e2e_$$.db"
SERVER_PID=""

cleanup() {
    if [ -n "$SERVER_PID" ]; then
        echo "🛑 Stopping ephemeral test server (PID: ${SERVER_PID})..."
        kill "${SERVER_PID}" 2>/dev/null || true
        wait "${SERVER_PID}" 2>/dev/null || true
    fi
    rm -f "${TMP_DB}"*
}
trap cleanup EXIT INT TERM

# Detect server binary
SERVER_BIN=""
if [ -f "./target/release/obaf-server" ]; then
    SERVER_BIN="./target/release/obaf-server"
elif [ -f "./target/debug/obaf-server" ]; then
    SERVER_BIN="./target/debug/obaf-server"
fi

if curl -s -f "${BASE_URL}/health" >/dev/null 2>&1; then
    echo "ℹ️ Existing OBAF server detected on ${BASE_URL}. Running e2e tests against live instance..."
else
    echo "🚀 Starting automated ephemeral OBAF server on port ${PORT}..."
    if [ -n "$SERVER_BIN" ]; then
        DATABASE_URL="sqlite://${TMP_DB}?mode=rwc" PORT="${PORT}" "$SERVER_BIN" > /tmp/obaf_e2e_server.log 2>&1 &
        SERVER_PID=$!
    else
        DATABASE_URL="sqlite://${TMP_DB}?mode=rwc" PORT="${PORT}" cargo run -p obaf-server > /tmp/obaf_e2e_server.log 2>&1 &
        SERVER_PID=$!
    fi

    # Wait for server to become healthy
    MAX_ATTEMPTS=40
    ATTEMPT=0
    until curl -s -f "${BASE_URL}/health" >/dev/null 2>&1; do
        ATTEMPT=$((ATTEMPT + 1))
        if [ "$ATTEMPT" -ge "$MAX_ATTEMPTS" ]; then
            echo "❌ Timed out waiting for OBAF server to become ready."
            if [ -f /tmp/obaf_e2e_server.log ]; then
                echo "Server logs:"
                cat /tmp/obaf_e2e_server.log
            fi
            exit 1
        fi
        sleep 0.25
    done
    echo "✅ Ephemeral test server ready on PID ${SERVER_PID}."
fi

echo "🧪 Running End-to-End Multi-Domain Test Suites..."
python3 tests/test_flow.py
python3 tests/test_pharmacy.py
python3 tests/test_retail_and_hotel.py
python3 tests/test_services_gym_bookings.py
python3 tests/test_webhooks_and_invoices.py
python3 tests/test_procurement_and_promotions_e2e.py

echo "🚀 Verifying Reference Quickstarts..."
python3 examples/quickstart_python/main.py
npx -y tsx examples/quickstart_typescript/index.ts

echo "🎉 All End-to-End Test Suites and Quickstart Validations passed successfully!"
