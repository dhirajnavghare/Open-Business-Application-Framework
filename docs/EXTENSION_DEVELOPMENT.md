# Domain Extension & Preset Development Guide

The **Open Business Application Framework (OBAF)** is engineered as a pluggable backend. Instead of requiring new database tables or fork-and-modify patterns for each new industry, developers can author **Domain Presets** and extensions that leverage OBAF's universal primitives.

---

## 🏛️ The Extension Architecture

OBAF decouples industry-specific logic from the high-performance core using three contracts:

```
┌────────────────────────────────────────────────────────┐
│                   Domain Preset JSON                   │
│   (Modules, Workflow States, Tax Slabs, Custom Schemas) │
└───────────────────────────┬────────────────────────────┘
                            │ bootstrapped into
┌───────────────────────────▼────────────────────────────┐
│               Universal Business Engines               │
│   (Catalog, Stock Ledger, Orders, Billing, Bookings)   │
└────────────────────────────────────────────────────────┘
```

---

## ⚡ Scaffolding a New Domain Preset

Use the `obaf-cli` tool to scaffold a new domain preset:

```bash
# Scaffold a preset for Dental Clinic
cargo run -p obaf-cli -- new-preset dental_clinic --title "Dental & Orthodontics Practice"
```

This creates `config/presets/dental_clinic.json`:

```json
{
  "preset_id": "dental_clinic",
  "name": "Dental & Orthodontics Practice",
  "version": "1.0.0",
  "description": "Custom domain operations and workflow preset for Dental & Orthodontics Practice",
  "active_modules": [
    "customers",
    "bookings",
    "catalog",
    "orders",
    "billing",
    "payments"
  ],
  "default_tax_slabs": [
    { "name": "Exempt Healthcare", "rate": 0.00 }
  ],
  "workflow_states": [
    "appointment_scheduled",
    "checked_in",
    "in_procedure",
    "prescription_issued",
    "completed"
  ]
}
```

---

## 📋 Defining Custom Field Schemas

If your domain requires specialized fields (e.g., `tooth_number`, `prescription_ref`, `vehicle_vin`), specify them in your preset under `order_schema` or `catalog_schema`:

```json
{
  "order_schema": {
    "type": "object",
    "properties": {
      "tooth_number": { "type": "integer", "minimum": 1, "maximum": 32 },
      "dentist_name": { "type": "string" }
    },
    "required": ["tooth_number"]
  }
}
```

When an order is created via `POST /api/v1/orders`, OBAF validates the JSON metadata against this schema using `jsonschema`. If required fields are missing, the API rejects the request before writing to the database.

---

## 🧪 Testing Your Extension

1. Validate your preset with the CLI doctor:
   ```bash
   cargo run -p obaf-cli -- doctor
   ```
2. Verify it appears in the preset catalog:
   ```bash
   cargo run -p obaf-cli -- presets
   ```
3. Initialize a test organization with your preset:
   ```bash
   curl -X POST http://127.0.0.1:8080/api/v1/admin/setup/preset \
     -H "Content-Type: application/json" \
     -d '{
       "organization_name": "Smile Care Clinic",
       "slug": "smile-care",
       "preset": "dental_clinic",
       "admin_email": "dr.smith@smilecare.test",
       "admin_name": "Dr. Smith",
       "admin_password": "SecurePassword123!"
     }'
   ```

---

## 🧩 Authoring Rust Module Extensions (`ObafExtension`)

Beyond declarative presets, developers can build compiled Rust plugins by implementing the `ObafExtension` trait from `obaf-core`:

```rust
use obaf_core::{
    DomainEvent, ExtensionContext, ExtensionManifest, ObafExtension, Result,
};

pub struct LoyaltyRewardsExtension;

impl ObafExtension for LoyaltyRewardsExtension {
    fn manifest(&self) -> ExtensionManifest {
        ExtensionManifest {
            name: "loyalty-rewards".to_string(),
            version: "1.0.0".to_string(),
            description: "Calculates reward points on order completion".to_string(),
            author: "Contributor <dev@example.com>".to_string(),
            required_modules: vec!["orders".to_string(), "customers".to_string()],
        }
    }

    fn on_init(&self, _ctx: &ExtensionContext) -> Result<()> {
        println!("Loyalty rewards extension initialized!");
        Ok(())
    }

    fn on_event(&self, event: &DomainEvent<serde_json::Value>) -> Result<()> {
        if event.event_type == "order.completed" {
            // Process loyalty points credit
        }
        Ok(())
    }
}
```

A complete reference implementation is available in `examples/community_extension`.
