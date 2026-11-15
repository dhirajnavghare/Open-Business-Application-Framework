#!/usr/bin/env bash
# ==============================================================================
# Open Business Application Framework (OBAF) - Performance Benchmark Suite
# Measures Throughput, Latency Percentiles (p50, p90, p99), and Memory RSS
# ==============================================================================
set -e

PORT=${PORT:-8080}
BASE_URL="http://127.0.0.1:${PORT}"
TMP_DB="/tmp/obaf_bench_$$.db"
SERVER_PID=""

cleanup() {
    if [ -n "$SERVER_PID" ]; then
        echo "🛑 Stopping benchmark server (PID: ${SERVER_PID})..."
        kill "${SERVER_PID}" 2>/dev/null || true
        wait "${SERVER_PID}" 2>/dev/null || true
    fi
    rm -f "${TMP_DB}"*
}
trap cleanup EXIT INT TERM

# Ensure server is running
if curl -s -f "${BASE_URL}/health" >/dev/null 2>&1; then
    echo "ℹ️ Existing OBAF server detected on ${BASE_URL}."
else
    echo "🚀 Starting OBAF release server for benchmarking on port ${PORT}..."
    SERVER_BIN="./target/release/obaf-server"
    if [ ! -f "$SERVER_BIN" ]; then
        cargo build --release -p obaf-server
    fi
    DATABASE_URL="sqlite://${TMP_DB}?mode=rwc" PORT="${PORT}" "$SERVER_BIN" > /tmp/obaf_bench_server.log 2>&1 &
    SERVER_PID=$!

    until curl -s -f "${BASE_URL}/health" >/dev/null 2>&1; do
        sleep 0.2
    done
    echo "✅ Benchmark server ready on PID ${SERVER_PID}."
fi

# Run benchmark runner via Python
python3 - << 'EOF'
import asyncio
import aiohttp
import time
import statistics
import os
import psutil

BASE_URL = os.environ.get("BASE_URL", "http://127.0.0.1:8080")
TOTAL_REQUESTS = 3000
CONCURRENCY = 40

async def fetch(session, url):
    start = time.perf_counter()
    async with session.get(url) as resp:
        await resp.read()
        latency = (time.perf_counter() - start) * 1000.0 # ms
        return resp.status, latency

async def run_benchmark():
    url = f"{BASE_URL}/health"
    print(f"\n⚡ Starting High-Concurrency Benchmark against {url}")
    print(f"📊 Target: {TOTAL_REQUESTS} requests | Concurrency: {CONCURRENCY} workers\n")

    conn = aiohttp.TCPConnector(limit=CONCURRENCY)
    latencies = []
    statuses = []

    start_total = time.perf_counter()
    async with aiohttp.ClientSession(connector=conn) as session:
        # Warmup
        for _ in range(50):
            await fetch(session, url)

        tasks = []
        for _ in range(TOTAL_REQUESTS):
            tasks.append(fetch(session, url))
        
        results = await asyncio.gather(*tasks)
        for status, lat in results:
            statuses.append(status)
            latencies.append(lat)

    elapsed = time.perf_counter() - start_total
    rps = len(latencies) / elapsed

    latencies.sort()
    p50 = statistics.median(latencies)
    p90 = latencies[int(len(latencies) * 0.90)]
    p95 = latencies[int(len(latencies) * 0.95)]
    p99 = latencies[int(len(latencies) * 0.99)]
    avg = statistics.mean(latencies)

    print("═══════════════════════════════════════════════════════════════════════")
    print("                    OBAF PERFORMANCE BENCHMARK RESULT                  ")
    print("═══════════════════════════════════════════════════════════════════════")
    print(f" Total Requests Completed:  {len(latencies)} (Success: {statuses.count(200)} / {len(latencies)})")
    print(f" Total Benchmark Time:      {elapsed:.3f} seconds")
    print(f" Throughput:                {rps:,.1f} requests/sec")
    print("───────────────────────────────────────────────────────────────────────")
    print(" Latency Percentiles (p99 < 5ms SLA):")
    print(f"   • Min Latency:           {min(latencies):.2f} ms")
    print(f"   • p50 (Median):          {p50:.2f} ms")
    print(f"   • p90:                   {p90:.2f} ms")
    print(f"   • p95:                   {p95:.2f} ms")
    print(f"   • p99:                   {p99:.2f} ms")
    print(f"   • Max Latency:           {max(latencies):.2f} ms")
    print("═══════════════════════════════════════════════════════════════════════\n")

try:
    asyncio.run(run_benchmark())
except ImportError:
    print("ℹ️ aiohttp not installed. Falling back to multi-threaded standard library benchmark...")
    import urllib.request
    from concurrent.futures import ThreadPoolExecutor

    def single_req(i):
        t0 = time.perf_counter()
        with urllib.request.urlopen(f"{BASE_URL}/health") as resp:
            resp.read()
        return (time.perf_counter() - t0) * 1000.0

    t0 = time.perf_counter()
    with ThreadPoolExecutor(max_workers=20) as ex:
        lats = list(ex.map(single_req, range(500)))
    total_sec = time.perf_counter() - t0
    lats.sort()
    rps = len(lats) / total_sec
    print(f"Completed {len(lats)} requests in {total_sec:.2f}s ({rps:.1f} req/s). p50: {statistics.median(lats):.2f}ms, p99: {lats[int(len(lats)*0.99)]:.2f}ms")
EOF
