# OBAF Localization & Regulatory Adaptation Architecture

*Engineered in alignment with Section 15 of the Master Project Vision & Architecture.*

---

## 1. Principles of Localization in OBAF

1. **Universal Financial Core**: The core accounting and calculation engine never hardcodes country-specific legal rules into shared business tables.
2. **Jurisdiction-Specific Adapters**: Regional compliance (such as Indian GST, European VAT, US Sales Tax) is implemented via pluggable configuration presets and declarative rules.
3. **UTC Everywhere**: All timestamps in the database and API payloads are stored and serialized strictly in ISO-8601 UTC (`chrono::DateTime<Utc>`). Timezone conversion occurs on presentation surfaces.
4. **128-Bit Decimal Precision**: Currency values use `rust_decimal::Decimal` with ISO-4217 currency identifiers, completely preventing floating-point inaccuracies.

---

## 2. Multi-Currency & ISO-4217 Currency Support

OBAF models monetary values using the `obaf_core::Money` struct:

```rust
use obaf_core::{Currency, Money};
use rust_decimal::Decimal;

let price = Money::new(Decimal::from(250), Currency::INR);
let usd_price = Money::new(Decimal::new(1999, 2), Currency::USD); // $19.99
```

Supported standard currencies include:
- `USD` (United States Dollar)
- `EUR` (Euro)
- `GBP` (British Pound Sterling)
- `INR` (Indian Rupee)
- `AUD` (Australian Dollar)
- `CAD` (Canadian Dollar)
- `SGD` (Singapore Dollar)
- `AED` (United Arab Emirates Dirham)
- `JPY` (Japanese Yen - 0 decimal places)

---

## 3. Tax Slabs & Regional Invoicing

### 3.1 Dual-Tax Split (GST)
For jurisdictions utilizing split goods & services tax (e.g. India CGST + SGST or Canada GST + PST):
- **Intra-State**: Tax rate is split evenly between Central (CGST) and State (SGST).
  $$\text{CGST} = \text{SGST} = \frac{\text{Taxable Amount} \times \text{Tax Rate}}{2}$$
- **Inter-State**: Full rate applied as Integrated Tax (IGST).

### 3.2 Single VAT / Sales Tax
For single-tier value added tax (e.g. UK, EU, UAE):
$$\text{Tax Amount} = \text{Taxable Amount} \times \text{VAT Rate}$$

Tested and validated via `/api/v1/billing/calculate-tax`.

---

## 4. Units of Measure (UoM) & Barcodes

For grocery, retail, and manufacturing domains, products define standard units of measure:
- `UNIT` / `PIECE` (Standard items)
- `KG` / `GRAM` (Weighed produce)
- `LITER` / `ML` (Liquids & beverages)
- `BOX` / `PACK` (Packaged multi-packs)
- `STRIP` / `VIAL` (Pharmaceutical dispensing)

All unit measurements support exact decimal quantities (e.g., $1.750\text{ kg}$).
