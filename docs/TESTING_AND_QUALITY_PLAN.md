# OBAF Testing & Quality Assurance Plan

## 1. Testing Pyramid

OBAF enforces automated quality verification across four distinct layers:

```
        ┌────────────────────────────────┐
        │   Live E2E Integration Tests   │ (Python test suites against live server)
        ├────────────────────────────────┤
        │    Concurrency & Load Tests    │ (benchmark_concurrency.py: 2800+ req/s)
        ├────────────────────────────────┤
        │    Crate Integration Tests     │ (saas_and_sync, tenant_isolation, etc.)
        ├────────────────────────────────┤
        │       Rust Unit Tests          │ (rules, billing_math, schema_validation)
        └────────────────────────────────┘
```

---

## 2. CI/CD Release Gates

Every pull request and commit must pass:
1. `cargo fmt --all --check`
2. `cargo check --workspace --all-targets`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace` (all 14+ unit/integration tests)
5. Docker multi-stage build check (`Dockerfile`)

---

## 3. End-to-End Domain Test Coverage

- **`tests/test_flow.py`**: Restaurant POS table management, KOT workflow, split bills, thermal receipt generation.
- **`tests/test_pharmacy.py`**: Batch numbers, medicine expiry tracking, GST tax slabs, prescription dispensing.
- **`tests/test_services_gym_bookings.py`**: Gym subscriptions, check-in attendance, session reservations, EOD settlement summaries.
- **`tests/test_webhooks_and_invoices.py`**: Real-time webhook dispatch, HMAC signatures, HTML printable invoice rendering.
- **`tests/test_retail_and_hotel.py`**: Barcode FMCG retail POS checkout, hotel suite reservations, guest check-in/out lifecycles.
- **`tests/benchmark_concurrency.py`**: Multi-worker concurrent load verification.
