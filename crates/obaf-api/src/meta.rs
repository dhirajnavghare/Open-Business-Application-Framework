use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct EngineInfoResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub supported_databases: Vec<&'static str>,
    pub available_presets: Vec<&'static str>,
    pub active_engines: Vec<&'static str>,
}

#[derive(Serialize, ToSchema)]
pub struct SchemaField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub ui_component: String,
    pub description: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct EntitySchema {
    pub entity: String,
    pub display_name: String,
    pub primary_key: String,
    pub fields: Vec<SchemaField>,
    pub actions: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SchemaDiscoveryResponse {
    pub entities: Vec<EntitySchema>,
}

#[utoipa::path(
    get,
    path = "/api/v1/meta/info",
    tag = "Meta",
    responses(
        (status = 200, description = "Framework engine metadata and active configurations", body = EngineInfoResponse)
    )
)]
pub async fn engine_info() -> Json<EngineInfoResponse> {
    Json(EngineInfoResponse {
        name: "Open Business Application Framework (OBAF)",
        version: env!("CARGO_PKG_VERSION"),
        description: "Headless, multi-domain business operations & billing engine in Rust",
        supported_databases: vec!["sqlite", "postgres", "mysql"],
        available_presets: vec![
            "restaurant_pos",
            "pharmacy",
            "clinic_opd",
            "coworking_space",
            "equipment_rental",
            "field_service",
            "gym_fitness",
            "laundry",
            "repair_shop",
            "cloud_kitchen_tiffin",
            "banquet_hall",
            "table_waitlist",
            "freelance_invoice",
            "pg_hostel",
            "visitor_pass",
            "salon_spa",
            "retail_grocery",
            "hotel_management",
            "education_coaching",
            "manufacturing_jobwork",
            "courier_fleet",
            "dairy_milk_delivery",
        ],
        active_engines: vec![
            "identity_rbac",
            "customers",
            "suppliers",
            "catalog",
            "inventory_ledger",
            "procurement_purchase_orders",
            "orders_workflow",
            "payments_settlement",
            "pos_cash_drawer",
            "promotions_coupons",
            "bookings_reservations",
            "subscriptions",
            "billing_tax",
            "credit_notes",
            "multi_currency_exchange",
            "reports_analytics",
            "webhooks_dispatch",
            "websocket_events",
        ],
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/meta/schema",
    tag = "Meta",
    responses(
        (status = 200, description = "Dynamic schema discovery for headless admin frontends", body = SchemaDiscoveryResponse)
    )
)]
pub async fn schema_discovery() -> Json<SchemaDiscoveryResponse> {
    // Foundational entity definitions for decoupled Admin UI introspection
    let entities = vec![
        EntitySchema {
            entity: "organization".to_string(),
            display_name: "Organization / Tenant".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Trading name of the business".to_string()),
                },
                SchemaField {
                    name: "slug".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Unique URL-friendly tenant identifier".to_string()),
                },
                SchemaField {
                    name: "preset".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "select_dropdown".to_string(),
                    description: Some("Domain preset configuration".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "view".to_string(),
                "create".to_string(),
                "update".to_string(),
            ],
        },
        EntitySchema {
            entity: "user".to_string(),
            display_name: "Staff & Users".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "email".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "email_input".to_string(),
                    description: Some("Work email for login".to_string()),
                },
                SchemaField {
                    name: "full_name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Full name of the user".to_string()),
                },
                SchemaField {
                    name: "role".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "role_select".to_string(),
                    description: Some("RBAC role (admin, manager, staff, cashier)".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "create".to_string(),
                "edit".to_string(),
                "deactivate".to_string(),
            ],
        },
        EntitySchema {
            entity: "customer".to_string(),
            display_name: "Customers & Clients".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Customer or company name".to_string()),
                },
                SchemaField {
                    name: "email".to_string(),
                    field_type: "string".to_string(),
                    required: false,
                    ui_component: "email_input".to_string(),
                    description: Some("Customer email address".to_string()),
                },
                SchemaField {
                    name: "phone".to_string(),
                    field_type: "string".to_string(),
                    required: false,
                    ui_component: "phone_input".to_string(),
                    description: Some("Contact phone number".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "view".to_string(),
                "create".to_string(),
                "update".to_string(),
            ],
        },
        EntitySchema {
            entity: "product".to_string(),
            display_name: "Catalog & Items".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "sku".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Stock keeping unit or bar code".to_string()),
                },
                SchemaField {
                    name: "name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Item or service name".to_string()),
                },
                SchemaField {
                    name: "price".to_string(),
                    field_type: "decimal".to_string(),
                    required: true,
                    ui_component: "currency_input".to_string(),
                    description: Some("Selling price".to_string()),
                },
                SchemaField {
                    name: "tax_rate".to_string(),
                    field_type: "decimal".to_string(),
                    required: true,
                    ui_component: "number_input".to_string(),
                    description: Some("Applicable tax rate (e.g. 0.05, 0.18)".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "view".to_string(),
                "create".to_string(),
                "update".to_string(),
            ],
        },
        EntitySchema {
            entity: "order".to_string(),
            display_name: "Orders & Tickets".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "order_number".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Unique sequential order identifier".to_string()),
                },
                SchemaField {
                    name: "status".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "status_badge".to_string(),
                    description: Some("Order lifecycle status".to_string()),
                },
                SchemaField {
                    name: "total_amount".to_string(),
                    field_type: "decimal".to_string(),
                    required: true,
                    ui_component: "currency_input".to_string(),
                    description: Some("Total order amount including tax".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "view".to_string(),
                "create".to_string(),
                "update_status".to_string(),
                "receipt".to_string(),
                "invoice_html".to_string(),
            ],
        },
        EntitySchema {
            entity: "booking".to_string(),
            display_name: "Bookings & Reservations".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "resource_name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Table, room, slot or resource allocated".to_string()),
                },
                SchemaField {
                    name: "start_time".to_string(),
                    field_type: "datetime".to_string(),
                    required: true,
                    ui_component: "datetime_picker".to_string(),
                    description: Some("Booking start time".to_string()),
                },
                SchemaField {
                    name: "end_time".to_string(),
                    field_type: "datetime".to_string(),
                    required: true,
                    ui_component: "datetime_picker".to_string(),
                    description: Some("Booking end time".to_string()),
                },
                SchemaField {
                    name: "status".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "status_badge".to_string(),
                    description: Some("CONFIRMED, CHECKED_IN, COMPLETED, CANCELLED".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "create".to_string(),
                "check_in".to_string(),
                "check_out".to_string(),
            ],
        },
        EntitySchema {
            entity: "subscription".to_string(),
            display_name: "Subscriptions & Memberships".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "plan_name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Plan or package tier".to_string()),
                },
                SchemaField {
                    name: "billing_cycle".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "select_dropdown".to_string(),
                    description: Some("MONTHLY, DAILY, ANNUAL".to_string()),
                },
                SchemaField {
                    name: "status".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "status_badge".to_string(),
                    description: Some("ACTIVE, PAUSED, EXPIRED, CANCELLED".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "create".to_string(),
                "update_status".to_string(),
                "check_in".to_string(),
            ],
        },
        EntitySchema {
            entity: "webhook".to_string(),
            display_name: "Webhooks & Automations".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "url".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "url_input".to_string(),
                    description: Some("Target webhook HTTPS URL".to_string()),
                },
                SchemaField {
                    name: "event_types".to_string(),
                    field_type: "array".to_string(),
                    required: true,
                    ui_component: "tag_input".to_string(),
                    description: Some("Subscribed event topics".to_string()),
                },
                SchemaField {
                    name: "is_active".to_string(),
                    field_type: "boolean".to_string(),
                    required: true,
                    ui_component: "toggle_switch".to_string(),
                    description: Some("Active status of delivery".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "create".to_string(),
                "test_ping".to_string(),
            ],
        },
        EntitySchema {
            entity: "supplier".to_string(),
            display_name: "Suppliers & Vendors".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Supplier trade name or legal business entity".to_string()),
                },
                SchemaField {
                    name: "contact_person".to_string(),
                    field_type: "string".to_string(),
                    required: false,
                    ui_component: "text_input".to_string(),
                    description: Some("Key account manager or representative".to_string()),
                },
                SchemaField {
                    name: "email".to_string(),
                    field_type: "string".to_string(),
                    required: false,
                    ui_component: "email_input".to_string(),
                    description: Some("Procurement or accounts payable email".to_string()),
                },
                SchemaField {
                    name: "phone".to_string(),
                    field_type: "string".to_string(),
                    required: false,
                    ui_component: "phone_input".to_string(),
                    description: Some("Direct phone number".to_string()),
                },
            ],
            actions: vec!["list".to_string(), "create".to_string(), "view".to_string()],
        },
        EntitySchema {
            entity: "stock_movement".to_string(),
            display_name: "Inventory Ledger Movements".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "product_id".to_string(),
                    field_type: "uuid".to_string(),
                    required: true,
                    ui_component: "product_select".to_string(),
                    description: Some("Referenced product".to_string()),
                },
                SchemaField {
                    name: "movement_type".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "select_dropdown".to_string(),
                    description: Some(
                        "Movement direction (PURCHASE, SALE, RETURN, ADJUSTMENT, LOSS)".to_string(),
                    ),
                },
                SchemaField {
                    name: "quantity".to_string(),
                    field_type: "decimal".to_string(),
                    required: true,
                    ui_component: "number_input".to_string(),
                    description: Some("Signed quantity moved".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "create".to_string(),
                "balance".to_string(),
            ],
        },
        EntitySchema {
            entity: "api_key".to_string(),
            display_name: "Machine API Keys & Hardware Credentials".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some(
                        "Descriptive label (e.g. Counter-1 POS Terminal, Kitchen Printer)"
                            .to_string(),
                    ),
                },
                SchemaField {
                    name: "permissions".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "select_dropdown".to_string(),
                    description: Some(
                        "Permission scope (read_only, read_write, admin)".to_string(),
                    ),
                },
            ],
            actions: vec![
                "list".to_string(),
                "create".to_string(),
                "revoke".to_string(),
            ],
        },
        EntitySchema {
            entity: "stock_location".to_string(),
            display_name: "Warehouses & Storage Locations".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Warehouse or location name".to_string()),
                },
                SchemaField {
                    name: "code".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some(
                        "Short unique location code (e.g. WH-01, POS-BAR)".to_string(),
                    ),
                },
            ],
            actions: vec!["list".to_string(), "create".to_string()],
        },
        EntitySchema {
            entity: "pos_shift".to_string(),
            display_name: "POS Registers & Cashier Shifts".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "register_name".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("POS Terminal or Register name".to_string()),
                },
                SchemaField {
                    name: "opening_float".to_string(),
                    field_type: "decimal".to_string(),
                    required: true,
                    ui_component: "currency_input".to_string(),
                    description: Some("Opening cash float in drawer".to_string()),
                },
                SchemaField {
                    name: "status".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "status_badge".to_string(),
                    description: Some("OPEN or CLOSED".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "open".to_string(),
                "current".to_string(),
                "close".to_string(),
            ],
        },
        EntitySchema {
            entity: "credit_note".to_string(),
            display_name: "Credit Notes & Returns".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "credit_note_number".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Sequential credit note ID".to_string()),
                },
                SchemaField {
                    name: "amount".to_string(),
                    field_type: "decimal".to_string(),
                    required: true,
                    ui_component: "currency_input".to_string(),
                    description: Some("Credit note amount".to_string()),
                },
                SchemaField {
                    name: "reason".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Reason for issuance (refund, billing error)".to_string()),
                },
            ],
            actions: vec!["list".to_string(), "create".to_string(), "view".to_string()],
        },
        EntitySchema {
            entity: "purchase_order".to_string(),
            display_name: "Purchase Orders & Procurement".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "po_number".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Purchase order reference code".to_string()),
                },
                SchemaField {
                    name: "supplier_id".to_string(),
                    field_type: "uuid".to_string(),
                    required: true,
                    ui_component: "supplier_select".to_string(),
                    description: Some("Supplier vendor ID".to_string()),
                },
                SchemaField {
                    name: "status".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "status_badge".to_string(),
                    description: Some(
                        "SUBMITTED, PARTIALLY_RECEIVED, RECEIVED, or CANCELLED".to_string(),
                    ),
                },
                SchemaField {
                    name: "total_amount".to_string(),
                    field_type: "decimal".to_string(),
                    required: true,
                    ui_component: "currency_input".to_string(),
                    description: Some("Total PO amount including tax".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "create".to_string(),
                "view".to_string(),
                "receive".to_string(),
                "cancel".to_string(),
            ],
        },
        EntitySchema {
            entity: "coupon".to_string(),
            display_name: "Promotions & Coupons".to_string(),
            primary_key: "id".to_string(),
            fields: vec![
                SchemaField {
                    name: "code".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "text_input".to_string(),
                    description: Some("Promo code string".to_string()),
                },
                SchemaField {
                    name: "discount_type".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    ui_component: "select".to_string(),
                    description: Some("PERCENTAGE or FIXED_AMOUNT".to_string()),
                },
                SchemaField {
                    name: "discount_value".to_string(),
                    field_type: "decimal".to_string(),
                    required: true,
                    ui_component: "decimal_input".to_string(),
                    description: Some("Discount multiplier or flat amount".to_string()),
                },
                SchemaField {
                    name: "is_active".to_string(),
                    field_type: "boolean".to_string(),
                    required: true,
                    ui_component: "toggle_switch".to_string(),
                    description: Some("Coupon activation status".to_string()),
                },
            ],
            actions: vec![
                "list".to_string(),
                "create".to_string(),
                "validate".to_string(),
                "redeem".to_string(),
            ],
        },
    ];

    Json(SchemaDiscoveryResponse { entities })
}
