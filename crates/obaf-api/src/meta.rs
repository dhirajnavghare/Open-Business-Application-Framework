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
            "gym_fitness",
            "repair_shop",
            "laundry",
            "coworking",
            "freelance",
        ],
        active_engines: vec![
            "identity_rbac",
            "catalog",
            "inventory_ledger",
            "orders_workflow",
            "billing_tax",
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
    ];

    Json(SchemaDiscoveryResponse { entities })
}
