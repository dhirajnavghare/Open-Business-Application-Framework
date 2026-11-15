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
        ],
        active_engines: vec![
            "identity_rbac",
            "customers",
            "catalog",
            "inventory_ledger",
            "orders_workflow",
            "payments_settlement",
            "bookings_reservations",
            "subscriptions",
            "billing_tax",
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
    ];

    Json(SchemaDiscoveryResponse { entities })
}
