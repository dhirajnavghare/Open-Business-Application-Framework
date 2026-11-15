use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        // Health
        crate::health::health_check,
        crate::health::metrics_check,
        // Meta
        crate::meta::engine_info,
        crate::meta::schema_discovery,
        // Admin
        crate::admin::setup::bootstrap_preset,
        crate::admin::audit::list_audit_logs,
        crate::admin::tenants::provision_tenant,
        crate::admin::tenants::list_tenants,
        crate::admin::tenants::get_tenant_usage,
        // Auth
        crate::auth::handlers::login,
        crate::auth::handlers::register,
        crate::auth::handlers::me,
        crate::auth::api_keys::create_api_key,
        crate::auth::api_keys::list_api_keys,
        crate::auth::api_keys::revoke_api_key,
        // Customers
        crate::customers::handlers::create_customer,
        crate::customers::handlers::list_customers,
        crate::customers::handlers::get_customer,
        // Suppliers
        crate::suppliers::handlers::create_supplier,
        crate::suppliers::handlers::list_suppliers,
        crate::suppliers::handlers::get_supplier,
        // Catalog
        crate::catalog::handlers::create_product,
        crate::catalog::handlers::list_products,
        crate::catalog::handlers::get_product,
        // Inventory
        crate::inventory::handlers::record_movement,
        crate::inventory::handlers::list_movements,
        crate::inventory::handlers::get_stock_balance,
        crate::inventory::handlers::create_stock_location,
        crate::inventory::handlers::list_stock_locations,
        // Orders
        crate::orders::handlers::create_order,
        crate::orders::handlers::list_orders,
        crate::orders::handlers::update_status,
        // Payments
        crate::payments::handlers::record_payment,
        crate::payments::handlers::list_payments,
        crate::payments::handlers::daily_settlement,
        crate::payments::handlers::refund_payment,
        // Bookings
        crate::bookings::handlers::create_booking,
        crate::bookings::handlers::list_bookings,
        crate::bookings::handlers::check_in_booking,
        crate::bookings::handlers::check_out_booking,
        // Subscriptions
        crate::subscriptions::handlers::create_subscription,
        crate::subscriptions::handlers::list_subscriptions,
        crate::subscriptions::handlers::update_status,
        crate::subscriptions::handlers::check_in_member,
        // Webhooks
        crate::webhooks::handlers::register_webhook,
        crate::webhooks::handlers::list_webhooks,
        crate::webhooks::handlers::test_webhook,
        // Reports
        crate::reports::handlers::get_dashboard_metrics,
        crate::reports::handlers::get_kitchen_prep_sheet,
        crate::reports::handlers::get_sales_by_payment_method,
        crate::reports::handlers::export_orders_csv,
        crate::reports::handlers::export_customers_csv,
        crate::reports::handlers::export_inventory_csv,
        // Billing
        crate::billing::handlers::calculate_tax,
        crate::billing::handlers::apply_rules,
        crate::billing::handlers::split_bill,
        crate::billing::handlers::generate_receipt,
        crate::billing::handlers::render_html_invoice,
        crate::billing::handlers::issue_credit_note,
        crate::billing::handlers::list_credit_notes,
        crate::billing::handlers::get_credit_note,
        // POS Shifts
        crate::pos::handlers::open_shift,
        crate::pos::handlers::get_current_shift,
        crate::pos::handlers::close_shift,
        crate::pos::handlers::list_shifts,
        // Sync
        crate::sync::handlers::sync_push,
        crate::sync::handlers::sync_pull,
        // Search
        crate::search::handlers::global_search,
    ),
    components(
        schemas(
            crate::health::HealthResponse,
            crate::meta::EngineInfoResponse,
            crate::meta::SchemaDiscoveryResponse,
            crate::meta::SchemaField,
            crate::meta::EntitySchema,
            crate::admin::setup::BootstrapPresetRequest,
            crate::admin::setup::BootstrapPresetResponse,
            crate::admin::setup::ApiErrorResponse,
            crate::admin::audit::AuditLogResponse,
            crate::auth::handlers::LoginRequest,
            crate::auth::handlers::LoginResponse,
            crate::auth::handlers::RegisterUserRequest,
            crate::auth::handlers::UserProfileResponse,
            crate::auth::extractor::AuthErrorResponse,
            crate::auth::api_keys::CreateApiKeyRequest,
            crate::auth::api_keys::CreateApiKeyResponse,
            crate::auth::api_keys::ApiKeySummary,
            crate::customers::handlers::CreateCustomerRequest,
            crate::customers::handlers::CustomerResponse,
            crate::suppliers::handlers::CreateSupplierRequest,
            crate::suppliers::handlers::SupplierResponse,
            crate::catalog::handlers::CreateProductRequest,
            crate::catalog::handlers::ProductResponse,
            crate::inventory::handlers::RecordStockMovementRequest,
            crate::inventory::handlers::StockMovementResponse,
            crate::orders::handlers::CreateOrderRequest,
            crate::orders::handlers::CreateOrderItemRequest,
            crate::orders::handlers::OrderResponse,
            crate::orders::handlers::OrderItemResponse,
            crate::orders::handlers::UpdateOrderStatusRequest,
            crate::payments::handlers::RecordPaymentRequest,
            crate::payments::handlers::RefundPaymentRequest,
            crate::payments::handlers::PaymentResponse,
            crate::payments::handlers::DailySettlementSummary,
            crate::bookings::handlers::CreateBookingRequest,
            crate::bookings::handlers::BookingResponse,
            crate::subscriptions::handlers::CreateSubscriptionRequest,
            crate::subscriptions::handlers::SubscriptionResponse,
            crate::subscriptions::handlers::UpdateSubscriptionStatusRequest,
            crate::webhooks::handlers::CreateWebhookRequest,
            crate::webhooks::handlers::WebhookResponse,
            crate::billing::handlers::TaxCalculationRequest,
            crate::billing::handlers::TaxCalculationResponse,
            crate::billing::handlers::TaxComponent,
            crate::billing::handlers::SplitBillRequest,
            crate::billing::handlers::SplitBillResponse,
            crate::billing::handlers::SplitBillShare,
            crate::billing::handlers::ReceiptResponse,
            crate::reports::handlers::DashboardMetrics,
            crate::reports::handlers::KitchenPrepItem,
            crate::reports::handlers::PaymentMethodSummary,
            crate::admin::tenants::ProvisionTenantRequest,
            crate::admin::tenants::ProvisionTenantResponse,
            crate::admin::tenants::TenantSummary,
            crate::admin::tenants::TenantUsageResponse,
            crate::billing::handlers::ApplyRulesRequest,
            obaf_core::rules::PricingRule,
            obaf_core::rules::RuleCondition,
            obaf_core::rules::RuleAction,
            obaf_core::rules::RuleAdjustment,
            obaf_core::rules::RuleEvaluationResult,
            crate::sync::handlers::SyncAction,
            crate::sync::handlers::SyncActionResult,
            crate::sync::handlers::SyncPushRequest,
            crate::sync::handlers::SyncPushResponse,
            crate::sync::handlers::SyncPullResponse,
            crate::inventory::handlers::ProductStockBalance,
            crate::inventory::handlers::CreateStockLocationRequest,
            crate::inventory::handlers::StockLocationResponse,
            crate::billing::handlers::IssueCreditNoteRequest,
            crate::billing::handlers::CreditNoteResponse,
            crate::pos::handlers::OpenShiftRequest,
            crate::pos::handlers::CloseShiftRequest,
            crate::pos::handlers::PosShiftResponse,
            crate::search::handlers::ProductSearchResult,
            crate::search::handlers::CustomerSearchResult,
            crate::search::handlers::OrderSearchResult,
            crate::search::handlers::SupplierSearchResult,
            crate::search::handlers::GlobalSearchResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Liveness and readiness endpoints"),
        (name = "Auth", description = "JWT authentication, registration, and user profiles"),
        (name = "Customers", description = "Customer, client, patient, and subscriber management"),
        (name = "Suppliers", description = "Vendor profiles, procurement sources, and contact management"),
        (name = "Catalog", description = "Products, services, and dynamic custom schemas"),
        (name = "Inventory", description = "Stock ledger, movements, batch tracking, and expiry dates"),
        (name = "Orders", description = "Multi-step order processing, items, and workflow state machines"),
        (name = "Payments", description = "Payment recording, cash/card/UPI settlements, and EOD reports"),
        (name = "POS", description = "POS registers, cashier cash drawer shifts, and variance reconciliation"),
        (name = "Bookings", description = "Resource scheduling, table/room reservations, and appointments"),
        (name = "Subscriptions", description = "Membership plans, renewals, and recurring subscriptions"),
        (name = "Webhooks", description = "Real-time event webhooks for WhatsApp, Zapier, and automation"),
        (name = "Billing", description = "GST/VAT tax calculations, bill splitting, and invoice/receipt generation"),
        (name = "Reports", description = "Operational dashboards, kitchen prep sheets, and sales metrics"),
        (name = "Sync", description = "Edge and offline synchronization with batch idempotency"),
        (name = "Search", description = "Global omni-search across products, customers, orders, and suppliers"),
        (name = "Meta", description = "Dynamic introspection and schema discovery for headless admin panels"),
        (name = "Admin", description = "Platform management and domain preset bootstrapping")
    ),
    info(
        title = "Open Business Application Framework (OBAF) API",
        version = "0.1.0",
        description = "High-performance headless business operations, billing, and workflow engine in Rust.",
        license(name = "MIT OR Apache-2.0")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}
