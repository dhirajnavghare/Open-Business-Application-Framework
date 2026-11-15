# OBAF Business Use Cases & Preset Specification

This document details the **22 Out-of-the-Box Industry Domain Presets** and dynamic JSON schema configurations supported natively by the **Open Business Application Framework (OBAF)**.

---

## 🏛️ 1. Architecture of Domain Presets

In OBAF, business logic is decoupled from hardcoded database schemas using **Universal Business Engines** (`crates/obaf-api`) combined with **Declarative JSON Presets** (`config/presets/<preset_id>.json`).

When an organization bootstraps via `POST /api/v1/admin/setup/preset` or `obaf seed <preset_id>`, the framework configures:
1. **Active Module Flags**: Enables only relevant capabilities (e.g. `kot` for dining, `batches` for pharmacies, `subscriptions` for gyms).
2. **Workflow State Machine**: Sets allowed state transitions for orders, reservations, or job cards.
3. **Default Tax Slabs**: Initializes regional GST/VAT tax categories.
4. **Dynamic JSON Schemas**: Validates custom domain metadata on products, orders, customers, or bookings using `jsonschema`.

---

## 🍽️ 2. Hospitality, Food & Dining Presets

### 2.1 Restaurant & Bar Point of Sale (`restaurant_pos`)
- **Business Use Case**: Full-service dine-in restaurants, quick-service bistros, and bars managing live tables, kitchen tickets, and split bills.
- **Active Modules**: `catalog`, `orders`, `kot`, `tables`, `billing`, `inventory`
- **Workflow State Machine**: `PENDING → CONFIRMED → COOKING → READY → COMPLETED → CANCELLED`
- **Tax Slabs**: GST 5% (Restaurant dine-in), GST 18% (Alcohol / Air-conditioned bar)
- **Custom Schemas**:
  - `product.spice_level`: `Mild`, `Medium`, `Hot`
  - `product.is_vegetarian`: `boolean`
  - `order.table_number`: `string` (e.g. `Table #04`)

### 2.2 Hotel, Resort & Hospitality Operations (`hotel_management`)
- **Business Use Case**: Boutique hotels, resorts, and lodges managing guest rooms, reservations, minibar folios, and check-in/out lifecycles.
- **Active Modules**: `bookings`, `customers`, `catalog`, `orders`, `billing`, `payments`, `reports`
- **Workflow State Machine**: `reserved → checked_in → room_service_active → settlement_pending → checked_out → cancelled`
- **Tax Slabs**: Room Tariff GST 12%, Luxury Suite GST 18%
- **Custom Schemas**:
  - `booking.room_number`: `string`
  - `booking.room_type`: `Deluxe`, `Executive Suite`, `Ocean View`, `Standard`
  - `booking.guest_id_proof`: Government ID number or passport
  - `booking.num_adults`, `booking.num_children`, `booking.complimentary_breakfast`

### 2.3 Cloud Kitchen & Tiffin Subscription (`cloud_kitchen_tiffin`)
- **Business Use Case**: Meal delivery providers and ghost kitchens offering scheduled recurring daily meal drops with pause/resume capabilities.
- **Active Modules**: `subscriptions`, `orders`, `catalog`, `inventory`, `billing`, `reports`
- **Workflow State Machine**: `plan_active → scheduled → kitchen_prep → out_for_delivery → delivered → paused`
- **Tax Slabs**: Tiffin Delivery GST 5%
- **Custom Schemas**:
  - `subscription.meal_type`: `Breakfast`, `Lunch`, `Dinner`
  - `subscription.dietary_preference`: `Vegetarian`, `Vegan`, `Jain`, `Non-Veg`
  - `subscription.delivery_instructions`: Drop-off notes

### 2.4 Event & Banquet Hall Booking (`banquet_hall`)
- **Business Use Case**: Wedding halls, convention centers, and event spaces managing calendar dates, catering packages, and advance deposits.
- **Active Modules**: `bookings`, `customers`, `billing`, `payments`, `catalog`
- **Workflow State Machine**: `inquiry → advance_received → confirmed → in_session → balance_settled → completed → cancelled`
- **Tax Slabs**: Banquet Hall Rental GST 18%
- **Custom Schemas**:
  - `booking.hall_name`: `Grand Ballroom`, `Terrace Lawn`, `Conference Hall A`
  - `booking.event_type`: `Wedding`, `Corporate Conference`, `Birthday Gala`
  - `booking.expected_guests`: `integer`

### 2.5 Table Reservation & Waitlist Queue (`table_waitlist`)
- **Business Use Case**: High-volume dining establishments managing walk-in guest waitlists and table assignments.
- **Active Modules**: `bookings`, `customers`, `webhooks`
- **Workflow State Machine**: `waitlisted → notified → seated → no_show → cancelled`
- **Tax Slabs**: Exempt 0.00%
- **Custom Schemas**:
  - `booking.party_size`: `integer`
  - `booking.seating_preference`: `Indoor AC`, `Outdoor Patio`, `Bar Counter`

---

## 🩺 3. Healthcare & Wellness Presets

### 3.1 Pharmacy & Medical Store (`pharmacy`)
- **Business Use Case**: Retail pharmacies and dispensaries requiring batch-level traceability, expiration dating, and FEFO stock deduction.
- **Active Modules**: `catalog`, `inventory`, `batches`, `expiry_alerts`, `billing`, `prescriptions`
- **Workflow State Machine**: `prescribed → batched → billed → dispensed → cancelled`
- **Tax Slabs**: Life Saving Drugs GST 0%, Formulations GST 12%, Supplements GST 18%
- **Custom Schemas**:
  - `product.drug_composition`: Active pharmaceutical ingredient (API)
  - `product.schedule_h_rx_required`: `boolean`
  - `stock_movement.batch_number`: `string`
  - `stock_movement.expiry_date`: ISO-8601 UTC timestamp

### 3.2 Outpatient Clinic & Doctor Practice (`clinic_opd`)
- **Business Use Case**: General practitioners, pediatricians, and dentists managing appointment tokens, vitals, and consultation billing.
- **Active Modules**: `bookings`, `customers`, `catalog`, `orders`, `billing`
- **Workflow State Machine**: `token_issued → vitals_recorded → consulting → prescription_ready → billed → completed`
- **Tax Slabs**: Healthcare Services Exempt 0.00%
- **Custom Schemas**:
  - `booking.doctor_name`: `string`
  - `booking.symptoms`: Initial patient complaint
  - `customer.blood_group`: `A+`, `B+`, `O+`, `AB+`, `A-`, `B-`, `O-`, `AB-`

### 3.3 Hair Salon & Luxury Spa (`salon_spa`)
- **Business Use Case**: Salons and wellness centers managing stylist schedules, treatment packages, chair assignments, and tip settlements.
- **Active Modules**: `bookings`, `catalog`, `orders`, `billing`, `payments`, `customers`
- **Workflow State Machine**: `booked → arrived → in_service → billed → completed`
- **Tax Slabs**: Personal Care Services GST 18%
- **Custom Schemas**:
  - `booking.stylist_id`: `string`
  - `booking.station_number`: `string`
  - `order.tip_amount`: `decimal`

### 3.4 Gym & Fitness Club (`gym_fitness`)
- **Business Use Case**: Fitness centers and sports academies managing membership tiers, automated renewals, and RFID/biometric turnstile attendance.
- **Active Modules**: `subscriptions`, `customers`, `payments`, `bookings`, `reports`
- **Workflow State Machine**: `active → grace_period → frozen → expired → cancelled`
- **Tax Slabs**: Gym Membership GST 18%
- **Custom Schemas**:
  - `subscription.plan_tier`: `Silver`, `Gold`, `Platinum Annual`
  - `subscription.locker_assigned`: `string`
  - `customer.emergency_contact`: `string`

---

## 🛒 4. Retail & Commerce Presets

### 4.1 Supermarket & Grocery Retail (`retail_grocery`)
- **Business Use Case**: FMCG grocers and supermarkets with barcode scanning, scale integration, unit conversions, and inventory reordering.
- **Active Modules**: `catalog`, `inventory`, `customers`, `orders`, `billing`, `payments`, `webhooks`
- **Workflow State Machine**: `cart_active → scanned → tendered → paid → dispatched → returned`
- **Tax Slabs**: Exempt Staples 0%, Essential FMCG GST 5%, Packaged Foods GST 12%, General Goods GST 18%
- **Custom Schemas**:
  - `product.barcode`: EAN-13 or UPC barcode
  - `product.unit_of_measure`: `kg`, `g`, `liter`, `piece`, `pack`
  - `product.rack_location`: Aisle and shelf code

### 4.2 Freelance Invoice & Expense Tracker (`freelance_invoice`)
- **Business Use Case**: Independent consultants, software engineers, and agencies tracking billable hours, project milestones, and client invoices.
- **Active Modules**: `customers`, `orders`, `billing`, `payments`, `reports`
- **Workflow State Machine**: `draft → sent_to_client → partially_paid → paid → written_off`
- **Tax Slabs**: Professional Services GST 18%
- **Custom Schemas**:
  - `order.project_name`: `string`
  - `order.milestone_title`: `string`
  - `order.hourly_rate`: `decimal`

### 4.3 Dairy Farm & Milk Delivery (`dairy_milk_delivery`)
- **Business Use Case**: Dairies managing recurring morning/evening milk drops, bottle deposits, and monthly settlement statements.
- **Active Modules**: `subscriptions`, `customers`, `catalog`, `billing`, `inventory`
- **Workflow State Machine**: `subscribed → route_assigned → delivered → bottle_collected → monthly_billed`
- **Tax Slabs**: Fresh Milk Exempt 0.00%
- **Custom Schemas**:
  - `subscription.shift`: `Morning (06:00)`, `Evening (18:00)`
  - `subscription.quantity_liters`: Exact decimal (e.g. `1.50`)
  - `subscription.milk_variant`: `Full Cream Cow`, `Buffalo`, `Skimmed`

---

## 🔧 5. Repairs & Field Services Presets

### 5.1 Repair Shop & Service Center (`repair_shop`)
- **Business Use Case**: Smartphone, electronics, and automotive service centers tracking job cards, parts used, and technician labor.
- **Active Modules**: `orders`, `catalog`, `inventory`, `customers`, `billing`, `webhooks`
- **Workflow State Machine**: `intake → diagnostic → waiting_for_parts → under_repair → quality_check → ready_for_pickup → delivered`
- **Tax Slabs**: Spare Parts GST 18%, Labor Services GST 18%
- **Custom Schemas**:
  - `order.device_imei_or_vin`: Device serial or vehicle VIN
  - `order.issue_description`: Customer reported fault
  - `order.technician_notes`: Diagnosis and repair logs

### 5.2 Field Service & Maintenance (`field_service`)
- **Business Use Case**: HVAC, plumbing, solar, and elevator maintenance companies dispatching mobile technicians with digital sign-off.
- **Active Modules**: `orders`, `customers`, `bookings`, `inventory`, `billing`
- **Workflow State Machine**: `dispatched → on_site → parts_replaced → customer_signed → invoiced`
- **Tax Slabs**: Field Maintenance GST 18%
- **Custom Schemas**:
  - `order.service_address`: Site location
  - `order.technician_id`: Assigned engineer
  - `order.customer_signature_data`: Base64 signature vector

### 5.3 Laundry & Dry-Cleaning (`laundry`)
- **Business Use Case**: Laundromats and dry-cleaners tracking individual garment tags, wash/iron phases, and automated SMS alerts.
- **Active Modules**: `orders`, `customers`, `billing`, `webhooks`, `payments`
- **Workflow State Machine**: `tagged → washing → dry_cleaning → pressing → bagged_ready → collected`
- **Tax Slabs**: Laundry Services GST 18%
- **Custom Schemas**:
  - `order.garment_tag_code`: Alphanumeric tag ID
  - `order.starch_level`: `None`, `Light`, `Medium`, `Heavy`

### 5.4 Equipment, Tool & Vehicle Rental (`equipment_rental`)
- **Business Use Case**: Construction tool, party equipment, and vehicle hire agencies managing security deposits and return inspections.
- **Active Modules**: `bookings`, `customers`, `catalog`, `billing`, `payments`, `orders`
- **Workflow State Machine**: `reserved → deposit_paid → released_on_hire → returned_inspected → deposit_refunded`
- **Tax Slabs**: Commercial Leasing GST 18%
- **Custom Schemas**:
  - `booking.asset_serial_number`: Equipment barcode or chassis ID
  - `booking.security_deposit_amount`: Exact decimal
  - `booking.fuel_level_at_checkout`: Percentage

---

## 🚚 6. Logistics, Real Estate & Facilities Presets

### 6.1 Courier Fleet & Logistics Dispatch (`courier_fleet`)
- **Business Use Case**: Last-mile courier services and logistics hubs tracking waybills, driver routes, proof-of-delivery (POD), and Cash on Delivery (COD).
- **Active Modules**: `orders`, `customers`, `payments`, `inventory`, `webhooks`
- **Workflow State Machine**: `waybill_created → sorted_at_hub → out_for_delivery → delivered_pod → cod_reconciled → return_to_origin`
- **Tax Slabs**: Freight Transportation GST 5%
- **Custom Schemas**:
  - `order.waybill_number`: Tracking barcode
  - `order.weight_kg`: Exact measured weight
  - `order.is_cod`: `boolean`
  - `order.cod_amount`: Decimal cash collection target

### 6.2 Co-Working & Shared Office Space (`coworking_space`)
- **Business Use Case**: Shared offices managing hot desks, dedicated team cabins, conference room booking credits, and high-speed Wi-Fi access passes.
- **Active Modules**: `subscriptions`, `bookings`, `customers`, `billing`, `payments`
- **Workflow State Machine**: `contract_signed → active → conference_credits_metered → renewal_due → terminated`
- **Tax Slabs**: Commercial Real Estate GST 18%
- **Custom Schemas**:
  - `subscription.seat_type`: `Hot Desk`, `Dedicated Desk`, `Private 4-Person Cabin`
  - `subscription.monthly_meeting_room_hours`: `integer`

### 6.3 PG & Student Hostel Rent & Utility Splitter (`pg_hostel`)
- **Business Use Case**: Hostels and paying guest (PG) properties managing room allocations, electricity sub-meter reading splits, and rent receipts.
- **Active Modules**: `customers`, `bookings`, `billing`, `payments`, `subscriptions`
- **Workflow State Machine**: `onboarded → room_assigned → utility_meter_read → monthly_rent_invoiced → paid`
- **Tax Slabs**: Residential Lodging Exempt 0.00%
- **Custom Schemas**:
  - `booking.room_and_bed_id`: e.g. `Room 204 - Bed B`
  - `billing.submeter_kwh_reading`: Current electricity meter reading
  - `billing.food_plan_included`: `boolean`

### 6.4 Visitor & Gate Pass Security (`visitor_pass`)
- **Business Use Case**: Gated residential societies and corporate offices issuing visitor security passes, contractor tracking, and vehicle permits.
- **Active Modules**: `customers`, `bookings`, `webhooks`, `reports`
- **Workflow State Machine**: `badge_generated → entry_scanned → on_premises → exit_scanned → overstay_alerted`
- **Tax Slabs**: Exempt 0.00%
- **Custom Schemas**:
  - `booking.visitor_photo_url`: Stored photo URL
  - `booking.host_employee_or_flat`: Destination contact
  - `booking.vehicle_number`: License plate string

---

## 🏭 7. Manufacturing & Education Presets

### 7.1 Manufacturing & Job Work (`manufacturing_jobwork`)
- **Business Use Case**: Machine shops, textile mills, and fabrication plants managing job cards, Bill of Materials (BOM), raw material consumption vs finished goods yield, and scrap scrap tracking.
- **Active Modules**: `catalog`, `inventory`, `orders`, `suppliers`, `billing`, `reports`
- **Workflow State Machine**: `job_card_opened → raw_material_issued → machining → quality_inspected → finished_goods_transferred → closed`
- **Tax Slabs**: Industrial Manufacturing GST 18%
- **Custom Schemas**:
  - `order.job_card_number`: Sequential production ticket
  - `order.input_raw_material_id`: Product UUID of sheet metal / raw stock
  - `order.scrap_weight_kg`: Production waste weight

### 7.2 Education Academy & Coaching Institute (`education_coaching`)
- **Business Use Case**: Coaching academies, test prep centers, and dance/music schools managing student enrollments, batches, installments, and attendance.
- **Active Modules**: `subscriptions`, `customers`, `bookings`, `billing`, `payments`
- **Workflow State Machine**: `enrolled → batch_allocated → active_attendance → installment_due → certified`
- **Tax Slabs**: Commercial Education Services GST 18%
- **Custom Schemas**:
  - `subscription.course_name`: `string`
  - `subscription.batch_timing`: e.g. `Mon-Wed-Fri 17:00-19:00`
  - `customer.guardian_phone`: Contact number for parents

---

## 💻 8. Developer CLI Workflow with Presets

### List All Available Domain Presets
```bash
cargo run -p obaf-cli -- presets
```

### Scaffold a New Domain Preset
```bash
cargo run -p obaf-cli -- new-preset boutique_jewelry --title "Luxury Boutique Jewelry"
```

### Seed a Tenant with Sample Data
```bash
cargo run -p obaf-cli -- seed retail_grocery
```
