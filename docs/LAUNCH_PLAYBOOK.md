# 🚀 OBAF World-Class Open-Source Viral Launch Playbook

This document is the actionable marketing, distribution, and community activation blueprint for the **Open Business Application Framework (OBAF)**. It translates deep market research across viral infrastructure projects (PocketBase, Supabase, Medusa.js, SurrealDB, Lago) into an exact execution strategy.

---

## 🎯 The Core Positioning & Hook

### The 1-Sentence Pitch
> **"OBAF is an ultra-fast, single-binary, headless business operations and billing engine built in Rust — think PocketBase, but engineered for commerce, POS, ERP, and precision billing with 22 industry presets and zero-daemon SQLite."**

### Why It Solves a Universal Pain Point
1. **The ERP Monolith Trap:** Legacy systems (Odoo, ERPNext) require 800MB–1.5GB of RAM to idle, multiple Python/Node daemons, and complex setup scripts.
2. **The Float/Penny Nightmare:** Generic databases and platforms store financial numbers as floats or integers without tax/split-bill pennies reconciliation. OBAF enforces strictly 128-bit `rust_decimal::Decimal` across all ledgers.
3. **The Micro-Kiosk Reality:** Small businesses and retail chains want an engine that runs natively on a \$35 Raspberry Pi or embedded edge POS terminal with offline batch synchronization (`/api/v1/sync`).

---

## 🏛️ Channel 1: Hacker News (Show HN)

### Submission Details
- **Title:** `Show HN: OBAF – Headless business operations and billing engine in Rust`
- **URL:** `https://github.com/dhirajnavghare/Open-Business-Application-Framework`
- **Timing:** Tuesday or Wednesday at 07:00–08:30 AM EST (12:00–13:30 UTC).

### First Comment from Author (Post Immediately After Submitting)
```markdown
Hey HN,

I built OBAF (Open Business Application Framework) because I was tired of spinning up massive, slow ERP monoliths (Odoo, ERPNext) just to handle simple commerce, POS, inventory, and billing workflows. Most of them take 1GB of RAM just to idle and require multiple runtime daemons.

OBAF delivers a single compiled Rust binary with:
• ~20MB memory footprint and < 3ms p99 latency (6,190+ req/s benchmarked locally).
• Zero-config embedded SQLite with WAL mode by default, or clustered PostgreSQL 16+ via SeaORM connection pooling.
• Exact 128-bit Decimal arithmetic (`rust_decimal`) with penny reconciliation on split bills, tax calculation, and discounts.
• 22 out-of-the-box business presets (Restaurant POS, Pharmacy FEFO, Retail Grocery, Hotel Management, Gym Memberships, Repair Job Cards, etc.).
• Dynamic JSON schema discovery (`/api/v1/meta/schema`) allowing modern frontends (React, Flutter, Next.js) to render dynamic forms and tables without hardcoding columns.
• Official zero-dependency TypeScript (`@obaf/sdk`) and Python (`obaf-sdk`) client SDKs.
• Full AI discovery support (`/llms.txt`) following the standard for Cursor and Claude Code.

Try it in 1 command:
$ curl -fsSL https://raw.githubusercontent.com/dhirajnavghare/Open-Business-Application-Framework/main/install.sh | bash
$ obaf init my-bistro --preset restaurant_pos

The code is dual-licensed under MIT and Apache-2.0. I'd love your feedback on the architecture, the 6 core primitives, and the database pooling abstraction!
```

---

## 🦀 Channel 2: Reddit (`r/rust` & `r/selfhosted`)

### Post for `r/rust`
- **Title:** `I built OBAF: A high-performance, single-binary business & billing engine in Axum + SeaORM`
- **Flairs:** `Project`, `Showcase`
- **Focus:** Technical deep dive into compile-time static assertions (`static_assertions`), Tokio event dispatching, SeaORM multi-DB routing (`TenantPoolRouter`), and exact Decimal math.

### Post for `r/selfhosted`
- **Title:** `A lightweight, single-binary alternative to Odoo / ERPNext with zero external dependencies (Docker + SQLite)`
- **Focus:** Low memory footprint (~20MB RAM), Raspberry Pi / VPS suitability, automated local backups (`obaf backup`), and 22 business presets.

---

## 🪶 Channel 3: Twitter / X Thread

```text
1/7 🚀 Introducing OBAF: The ultra-fast, single-binary business operations and billing engine built in Rust.

Think PocketBase, but engineered for commerce, POS, inventory, and financial operations.

github.com/dhirajnavghare/Open-Business-Application-Framework 🧵👇

2/7 Why did we build it?
Legacy ERPs (Odoo, ERPNext) are heavy:
❌ 1.2GB idle RAM
❌ Multi-daemon Python/Node setups
❌ Fragile floating-point money math

OBAF:
✅ ~20MB RAM footprint
✅ < 3ms p99 latency
✅ Strict 128-bit decimal arithmetic

3/7 Zero-Config SQLite or Clustered Postgres:
Run locally on your laptop or edge kiosk with zero setup. Scale to millions of transactions on clustered PostgreSQL 16+ with our built-in TenantPoolRouter.

4/7 22 Out-of-the-box Business Domain Presets:
🍕 Restaurant POS & KOT
💊 Pharmacy FEFO batch expiry
🛒 Retail FMCG Barcode checkout
🏨 Hotel folio reservations
🏋️ Gym memberships & booking slots

5/7 Developer Experience first:
Scaffold a production-ready stack in 1 line:
$ curl -fsSL https://raw.githubusercontent.com/.../install.sh | bash
$ obaf init my-store --preset retail_grocery

6/7 Polyglot & AI Native:
• Zero-dependency TypeScript SDK (@obaf/sdk)
• Zero-dependency Python SDK (obaf-sdk)
• Built-in /llms.txt standard for Cursor, Claude Code & AI agents

7/7 100% Open-Source (MIT & Apache-2.0).
⭐ Star on GitHub: github.com/dhirajnavghare/Open-Business-Application-Framework
```

---

## 😻 Channel 4: Product Hunt Launch

- **Tagline:** Ultra-fast, single-binary business & billing engine in Rust
- **Pricing:** Free / Open Source (Dual MIT / Apache 2.0)
- **Topics:** Developer Tools, Open Source, E-Commerce, Productivity, Rust
- **First Maker Comment:** Emphasize developer freedom, self-hosting privacy, and instant local bootstrapping.
