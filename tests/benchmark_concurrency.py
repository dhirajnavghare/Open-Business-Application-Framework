#!/usr/bin/env python3
"""
OBAF Production Concurrency & Throughput Benchmark
Measures latency (min, avg, p50, p95, p99, max) and error rates under concurrent load.
"""

import concurrent.futures
import json
import statistics
import sys
import time
import urllib.request

BASE_URL = "http://127.0.0.1:8080"
TOTAL_REQUESTS = 150
CONCURRENCY = 25

ENDPOINTS = [
    "/health",
    "/metrics",
    "/api/v1/meta/info",
    "/api/v1/meta/schema",
]

def make_request(path):
    url = f"{BASE_URL}{path}"
    start = time.perf_counter()
    try:
        req = urllib.request.Request(url)
        with urllib.request.urlopen(req, timeout=5) as response:
            status = response.getcode()
            body = response.read()
            elapsed_ms = (time.perf_counter() - start) * 1000.0
            return {
                "path": path,
                "status": status,
                "latency_ms": elapsed_ms,
                "bytes": len(body),
                "error": None
            }
    except Exception as e:
        elapsed_ms = (time.perf_counter() - start) * 1000.0
        return {
            "path": path,
            "status": 0,
            "latency_ms": elapsed_ms,
            "bytes": 0,
            "error": str(e)
        }

def run_benchmark():
    print(f"🚀 Benchmarking OBAF at {BASE_URL}...")
    print(f"📊 Total Requests: {TOTAL_REQUESTS} across {CONCURRENCY} concurrent workers")
    
    # Check if server is running
    try:
        with urllib.request.urlopen(f"{BASE_URL}/health", timeout=2) as r:
            if r.getcode() != 200:
                print("❌ Server /health check returned non-200")
                sys.exit(1)
    except Exception as e:
        print(f"❌ Server not reachable at {BASE_URL}. Ensure obaf-server is running.")
        sys.exit(1)

    tasks = []
    for i in range(TOTAL_REQUESTS):
        path = ENDPOINTS[i % len(ENDPOINTS)]
        tasks.append(path)

    start_total = time.perf_counter()
    results = []

    with concurrent.futures.ThreadPoolExecutor(max_workers=CONCURRENCY) as executor:
        futures = [executor.submit(make_request, p) for p in tasks]
        for f in concurrent.futures.as_completed(futures):
            results.append(f.result())

    total_time = time.perf_counter() - start_total
    rps = len(results) / total_time

    latencies = [r["latency_ms"] for r in results if r["error"] is None]
    errors = [r for r in results if r["error"] is not None]

    if not latencies:
        print("❌ All requests failed!")
        sys.exit(1)

    latencies.sort()
    min_lat = min(latencies)
    avg_lat = statistics.mean(latencies)
    p50_lat = statistics.median(latencies)
    p95_lat = latencies[int(len(latencies) * 0.95)]
    p99_lat = latencies[int(len(latencies) * 0.99)]
    max_lat = max(latencies)

    print("\n" + "=" * 50)
    print("🎯 OBAF CONCURRENCY BENCHMARK RESULTS")
    print("=" * 50)
    print(f"• Completed Requests: {len(results)}")
    print(f"• Total Time:         {total_time:.2f}s")
    print(f"• Throughput:         {rps:.1f} req/sec")
    print(f"• Error Rate:         {len(errors)}/{len(results)} ({len(errors)/len(results)*100:.1f}%)")
    print("-" * 50)
    print(f"• Min Latency:        {min_lat:.2f} ms")
    print(f"• Mean Latency:       {avg_lat:.2f} ms")
    print(f"• P50 (Median):       {p50_lat:.2f} ms")
    print(f"• P95 Latency:        {p95_lat:.2f} ms")
    print(f"• P99 Latency:        {p99_lat:.2f} ms")
    print(f"• Max Latency:        {max_lat:.2f} ms")
    print("=" * 50)

    if errors:
        print("❌ Errors detected during benchmark:")
        for err in errors[:5]:
            print(f"  - {err['path']}: {err['error']}")
        sys.exit(1)
    else:
        print("✅ Production Hardening Benchmark Passed! Zero errors under concurrent load.")

if __name__ == "__main__":
    run_benchmark()
