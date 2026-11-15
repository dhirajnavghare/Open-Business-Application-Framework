use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        // Health
        crate::health::health_check,
        // Meta
        crate::meta::engine_info,
        crate::meta::schema_discovery,
        // Admin
        crate::admin::setup::bootstrap_preset,
        crate::admin::audit::list_audit_logs,
        // Auth
        crate::auth::handlers::login,
        crate::auth::handlers::register,
        crate::auth::handlers::me,
        // Customers
        crate::customers::handlers::create_customer,
        crate::customers::handlers::list_customers,
        crate::customers::handlers::get_customer,
        // Catalog
        crate::catalog::handlers::create_product,
        crate::catalog::handlers::list_products,
        crate::catalog::handlers::get_product,
        // Inventory
        crate::inventory::handlers::record_movement,
        crate::inventory::handlers::list_movements,
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
        crate::billing::handlers::split_bill,
        crate::billing::handlers::generate_receipt,
        crate::billing::handlers::render_html_invoice,
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
            crate::customers::handlers::CreateCustomerRequest,
            crate::customers::handlers::CustomerResponse,
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
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Liveness and readiness endpoints"),
        (name = "Auth", description = "JWT authentication, registration, and user profiles"),
        (name = "Customers", description = "Customer, client, patient, and subscriber management"),
        (name = "Catalog", description = "Products, services, and dynamic custom schemas"),
        (name = "Inventory", description = "Stock ledger, movements, batch tracking, and expiry dates"),
        (name = "Orders", description = "Multi-step order processing, items, and workflow state machines"),
        (name = "Payments", description = "Payment recording, cash/card/UPI settlements, and EOD reports"),
        (name = "Bookings", description = "Resource scheduling, table/room reservations, and appointments"),
        (name = "Subscriptions", description = "Membership plans, renewals, and recurring subscriptions"),
        (name = "Webhooks", description = "Real-time event webhooks for WhatsApp, Zapier, and automation"),
        (name = "Billing", description = "GST/VAT tax calculations, bill splitting, and invoice/receipt generation"),
        (name = "Reports", description = "Operational dashboards, kitchen prep sheets, and sales metrics"),
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
