# OBAF Preset Specification & Dynamic Schemas

## 1. Domain Preset File Format

Presets reside in `config/presets/<preset_id>.json` and define domain configuration:

```json
{
  "preset_id": "restaurant_pos",
  "name": "Restaurant & Bar Point of Sale",
  "version": "1.0.0",
  "description": "Real-time table orders, kitchen tickets, split billing, and settlements",
  "active_modules": ["catalog", "orders", "kot", "tables", "billing", "inventory"],
  "default_tax_slabs": [
    { "name": "GST 5%", "rate": 0.05 },
    { "name": "GST 18%", "rate": 0.18 }
  ],
  "workflow_states": ["PENDING", "CONFIRMED", "COOKING", "READY", "COMPLETED", "CANCELLED"],
  "custom_fields": {
    "product": {
      "type": "object",
      "properties": {
        "spice_level": { "type": "string", "enum": ["Mild", "Medium", "Hot"] },
        "is_vegetarian": { "type": "boolean" }
      }
    }
  }
}
```

---

## 2. Dynamic Schema Validation

When creating or updating entities, custom attributes submitted in `metadata` are validated against the preset's JSON schema using `obaf_core::validate_metadata`:
- If invalid or missing required properties, the API rejects the request with HTTP `422 Unprocessable Entity`.
- Dynamic attributes are indexed and queryable without requiring DDL alterations.
