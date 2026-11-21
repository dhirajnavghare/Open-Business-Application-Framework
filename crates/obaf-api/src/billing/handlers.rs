use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use obaf_database::entities::{order, order_item, organization};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;
use obaf_core::rules::{evaluate_pricing_rules, PricingRule, RuleEvaluationResult};

#[derive(Deserialize, ToSchema)]
pub struct ApplyRulesRequest {
    pub subtotal: Decimal,
    pub promo_code: Option<String>,
    pub rules: Vec<PricingRule>,
}

#[utoipa::path(
    post,
    path = "/api/v1/billing/apply-rules",
    tag = "Billing",
    request_body = ApplyRulesRequest,
    responses(
        (status = 200, description = "Dynamic pricing rules, discounts and surcharges evaluated", body = RuleEvaluationResult)
    )
)]
pub async fn apply_rules(Json(payload): Json<ApplyRulesRequest>) -> Json<RuleEvaluationResult> {
    let result = evaluate_pricing_rules(
        &payload.rules,
        payload.subtotal,
        payload.promo_code.as_deref(),
    );
    Json(result)
}

#[derive(Deserialize, ToSchema)]
pub struct TaxCalculationRequest {
    pub taxable_amount: Decimal,
    pub tax_rate: Decimal, // e.g. 0.05, 0.18
    pub tax_type: String,  // "GST" or "VAT"
    pub is_interstate: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct TaxComponent {
    pub name: String,
    pub rate: Decimal,
    pub amount: Decimal,
}

#[derive(Serialize, ToSchema)]
pub struct TaxCalculationResponse {
    pub taxable_amount: Decimal,
    pub total_tax: Decimal,
    pub grand_total: Decimal,
    pub components: Vec<TaxComponent>,
}

#[derive(Deserialize, ToSchema)]
pub struct SplitBillRequest {
    pub total_amount: Decimal,
    pub num_parties: usize,
}

#[derive(Serialize, ToSchema)]
pub struct SplitBillShare {
    pub party_index: usize,
    pub share_amount: Decimal,
}

#[derive(Serialize, ToSchema)]
pub struct SplitBillResponse {
    pub total_amount: Decimal,
    pub num_parties: usize,
    pub shares: Vec<SplitBillShare>,
}

#[derive(Serialize, ToSchema)]
pub struct ReceiptResponse {
    pub order_number: String,
    pub receipt_text: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/billing/calculate-tax",
    tag = "Billing",
    request_body = TaxCalculationRequest,
    responses(
        (status = 200, description = "Exact tax breakdown computed", body = TaxCalculationResponse)
    )
)]
pub async fn calculate_tax(
    Json(payload): Json<TaxCalculationRequest>,
) -> Json<TaxCalculationResponse> {
    let total_tax = payload.taxable_amount * payload.tax_rate;
    let grand_total = payload.taxable_amount + total_tax;
    let mut components = Vec::new();

    if payload.tax_type.to_uppercase() == "GST" {
        if payload.is_interstate.unwrap_or(false) {
            components.push(TaxComponent {
                name: "IGST".to_string(),
                rate: payload.tax_rate,
                amount: total_tax,
            });
        } else {
            let half_rate = payload.tax_rate / Decimal::from(2);
            let half_tax = total_tax / Decimal::from(2);
            components.push(TaxComponent {
                name: "CGST".to_string(),
                rate: half_rate,
                amount: half_tax,
            });
            components.push(TaxComponent {
                name: "SGST".to_string(),
                rate: half_rate,
                amount: half_tax,
            });
        }
    } else {
        components.push(TaxComponent {
            name: "VAT".to_string(),
            rate: payload.tax_rate,
            amount: total_tax,
        });
    }

    Json(TaxCalculationResponse {
        taxable_amount: payload.taxable_amount,
        total_tax,
        grand_total,
        components,
    })
}

#[utoipa::path(
    post,
    path = "/api/v1/billing/split-bill",
    tag = "Billing",
    request_body = SplitBillRequest,
    responses(
        (status = 200, description = "Bill split evenly across parties with penny-rounding reconciliation", body = SplitBillResponse),
        (status = 400, description = "Number of parties must be at least 1", body = AuthErrorResponse)
    )
)]
pub async fn split_bill(
    Json(payload): Json<SplitBillRequest>,
) -> Result<Json<SplitBillResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    if payload.num_parties == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "num_parties must be greater than 0".to_string(),
            }),
        ));
    }

    let divisor = Decimal::from(payload.num_parties as i64);
    let base_share = (payload.total_amount / divisor).round_dp(2);
    let mut shares = Vec::with_capacity(payload.num_parties);
    let mut allocated = Decimal::ZERO;

    for i in 1..payload.num_parties {
        shares.push(SplitBillShare {
            party_index: i,
            share_amount: base_share,
        });
        allocated += base_share;
    }

    // Last party absorbs the penny difference so sum(shares) == total_amount
    let last_share = payload.total_amount - allocated;
    shares.push(SplitBillShare {
        party_index: payload.num_parties,
        share_amount: last_share,
    });

    Ok(Json(SplitBillResponse {
        total_amount: payload.total_amount,
        num_parties: payload.num_parties,
        shares,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/billing/orders/{id}/receipt",
    tag = "Billing",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Formatted ESC/POS compatible thermal receipt text", body = ReceiptResponse),
        (status = 404, description = "Order not found", body = AuthErrorResponse)
    )
)]
pub async fn generate_receipt(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ReceiptResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let o = order::Entity::find()
        .filter(order::Column::Id.eq(id))
        .filter(order::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Order not found".to_string(),
                }),
            )
        })?;

    let org = organization::Entity::find_by_id(o.tenant_id)
        .one(&state.db)
        .await
        .unwrap_or(None);

    let items = order_item::Entity::find()
        .filter(order_item::Column::OrderId.eq(o.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let org_name = org
        .map(|org| org.name)
        .unwrap_or_else(|| "STORE".to_string());
    let mut receipt = String::new();

    receipt.push_str("================================\n");
    receipt.push_str(&format!("         {}\n", org_name.to_uppercase()));
    receipt.push_str("================================\n");
    receipt.push_str(&format!("Order:  {}\n", o.order_number));
    if let Some(table) = o.table_or_room {
        receipt.push_str(&format!("Table/Room: {}\n", table));
    }
    receipt.push_str(&format!(
        "Date:   {}\n",
        o.created_at.format("%Y-%m-%d %H:%M:%S")
    ));
    receipt.push_str("--------------------------------\n");
    receipt.push_str("Item             Qty     Subtotal\n");
    receipt.push_str("--------------------------------\n");

    for item in items {
        let name_trimmed = if item.item_name.len() > 16 {
            &item.item_name[..16]
        } else {
            &item.item_name
        };
        receipt.push_str(&format!(
            "{:<16} {:>3} {:>10.2}\n",
            name_trimmed, item.quantity, item.subtotal
        ));
    }

    receipt.push_str("--------------------------------\n");
    receipt.push_str(&format!("Subtotal:             {:>10.2}\n", o.subtotal));
    receipt.push_str(&format!("Tax (GST/VAT):        {:>10.2}\n", o.tax_total));
    receipt.push_str(&format!("Grand Total:          {:>10.2}\n", o.grand_total));
    receipt.push_str("================================\n");
    receipt.push_str("    THANK YOU FOR YOUR VISIT!   \n");
    receipt.push_str("================================\n");

    Ok(Json(ReceiptResponse {
        order_number: o.order_number,
        receipt_text: receipt,
    }))
}

/// Helper to escape untrusted user inputs before interpolating into HTML templates (mitigates XSS)
fn html_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[utoipa::path(
    get,
    path = "/api/v1/billing/orders/{id}/invoice.html",
    tag = "Billing",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Printable, styled HTML invoice", content_type = "text/html"),
        (status = 404, description = "Order not found", body = AuthErrorResponse)
    )
)]
pub async fn render_html_invoice(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Html<String>, (StatusCode, Json<AuthErrorResponse>)> {
    let o = order::Entity::find()
        .filter(order::Column::Id.eq(id))
        .filter(order::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Order not found".to_string(),
                }),
            )
        })?;

    let org = organization::Entity::find_by_id(o.tenant_id)
        .one(&state.db)
        .await
        .unwrap_or(None);

    let items = order_item::Entity::find()
        .filter(order_item::Column::OrderId.eq(o.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let org_name = org
        .map(|org| org.name)
        .unwrap_or_else(|| "Business Invoice".to_string());
    let safe_org_name = html_escape(&org_name);
    let safe_order_number = html_escape(&o.order_number);
    let mut items_html = String::new();

    for item in items {
        items_html.push_str(&format!(
            "<tr><td style='padding: 8px; border-bottom: 1px solid #e2e8f0;'>{}</td><td style='padding: 8px; border-bottom: 1px solid #e2e8f0; text-align: center;'>{}</td><td style='padding: 8px; border-bottom: 1px solid #e2e8f0; text-align: right;'>${:.2}</td><td style='padding: 8px; border-bottom: 1px solid #e2e8f0; text-align: right;'>${:.2}</td></tr>",
            html_escape(&item.item_name), item.quantity, item.unit_price, item.subtotal
        ));
    }

    let customer_line = o
        .customer_name
        .map(|c| {
            format!(
                "<p style='margin: 0;'><strong>Billed To:</strong> {}</p>",
                html_escape(&c)
            )
        })
        .unwrap_or_default();
    let table_line = o
        .table_or_room
        .map(|t| {
            format!(
                "<p style='margin: 0;'><strong>Reference / Table:</strong> {}</p>",
                html_escape(&t)
            )
        })
        .unwrap_or_default();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Tax Invoice - {}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; background: #f8fafc; color: #1e293b; padding: 40px; margin: 0; }}
        .invoice-card {{ max-width: 650px; margin: 0 auto; background: #fff; padding: 36px; border-radius: 12px; box-shadow: 0 4px 6px -1px rgba(0,0,0,0.1); border: 1px solid #e2e8f0; }}
        .header {{ display: flex; justify-content: space-between; align-items: flex-start; border-bottom: 2px solid #0f172a; padding-bottom: 20px; margin-bottom: 24px; }}
        .badge {{ background: #e0f2fe; color: #0369a1; padding: 4px 10px; border-radius: 9999px; font-weight: 600; font-size: 13px; }}
        table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
        th {{ background: #f1f5f9; padding: 10px 8px; text-align: left; font-size: 13px; text-transform: uppercase; color: #475569; }}
        .summary {{ margin-top: 24px; display: flex; justify-content: flex-end; }}
        .summary-box {{ width: 260px; }}
        .summary-row {{ display: flex; justify-content: space-between; padding: 6px 0; font-size: 14px; }}
        .summary-total {{ font-size: 18px; font-weight: 700; color: #0f172a; border-top: 2px solid #0f172a; margin-top: 8px; padding-top: 8px; }}
        @media print {{ body {{ background: #fff; padding: 0; }} .invoice-card {{ box-shadow: none; border: none; padding: 0; }} }}
    </style>
</head>
<body>
    <div class="invoice-card">
        <div class="header">
            <div>
                <h1 style="margin: 0; font-size: 24px; color: #0f172a;">{}</h1>
                <p style="margin: 4px 0 0; color: #64748b; font-size: 14px;">Official Settlement Invoice</p>
            </div>
            <div style="text-align: right;">
                <span class="badge">PAID / SETTLED</span>
                <p style="margin: 8px 0 0; font-size: 14px; font-weight: 600;">{}</p>
                <p style="margin: 2px 0 0; font-size: 12px; color: #64748b;">{}</p>
            </div>
        </div>
        <div style="font-size: 14px; color: #334155; margin-bottom: 16px;">
            {}
            {}
        </div>
        <table>
            <thead>
                <tr>
                    <th>Item Description</th>
                    <th style="text-align: center;">Qty</th>
                    <th style="text-align: right;">Price</th>
                    <th style="text-align: right;">Total</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
        <div class="summary">
            <div class="summary-box">
                <div class="summary-row"><span>Subtotal</span><span>${:.2}</span></div>
                <div class="summary-row"><span>Tax (GST/VAT)</span><span>${:.2}</span></div>
                <div class="summary-row summary-total"><span>Grand Total</span><span>${:.2}</span></div>
            </div>
        </div>
        <div style="margin-top: 40px; text-align: center; border-top: 1px dashed #cbd5e1; padding-top: 16px; font-size: 12px; color: #94a3b8;">
            Generated by Open Business Application Framework (OBAF) • Thank you for your business!
        </div>
    </div>
</body>
</html>"#,
        safe_order_number,
        safe_org_name,
        safe_order_number,
        o.created_at.format("%B %d, %Y - %H:%M UTC"),
        customer_line,
        table_line,
        items_html,
        o.subtotal,
        o.tax_total,
        o.grand_total
    );

    Ok(axum::response::Html(html))
}

#[derive(Deserialize, ToSchema)]
pub struct IssueCreditNoteRequest {
    pub order_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub amount: Decimal,
    pub tax_amount: Decimal,
    pub reason: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreditNoteResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub credit_note_number: String,
    pub order_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub amount: Decimal,
    pub tax_amount: Decimal,
    pub reason: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<obaf_database::entities::credit_note::Model> for CreditNoteResponse {
    fn from(m: obaf_database::entities::credit_note::Model) -> Self {
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            credit_note_number: m.credit_note_number,
            order_id: m.order_id,
            customer_id: m.customer_id,
            amount: m.amount,
            tax_amount: m.tax_amount,
            reason: m.reason,
            status: m.status,
            created_at: m.created_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/billing/credit-notes",
    tag = "Billing",
    security(("bearer_auth" = [])),
    request_body = IssueCreditNoteRequest,
    responses(
        (status = 201, description = "Credit note issued successfully", body = CreditNoteResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn issue_credit_note(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<IssueCreditNoteRequest>,
) -> Result<(StatusCode, Json<CreditNoteResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    use obaf_database::entities::credit_note;
    use sea_orm::{ActiveModelTrait, Set};

    let cn_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let cn_number = format!("CN-{}", cn_id.to_string()[..8].to_uppercase());

    let new_cn = credit_note::ActiveModel {
        id: Set(cn_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        credit_note_number: Set(cn_number.clone()),
        order_id: Set(payload.order_id),
        customer_id: Set(payload.customer_id),
        amount: Set(payload.amount),
        tax_amount: Set(payload.tax_amount),
        reason: Set(payload.reason),
        status: Set("ISSUED".to_string()),
        created_at: Set(now),
    };

    let inserted = new_cn.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to issue credit note: {}", e),
            }),
        )
    })?;

    state.broadcast_tenant_event(
        auth.tenant_id.as_uuid(),
        "billing.credit_note_issued",
        serde_json::json!({
            "credit_note_id": cn_id,
            "credit_note_number": cn_number,
            "amount": payload.amount
        }),
    );

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/billing/credit-notes",
    tag = "Billing",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of credit notes", body = Vec<CreditNoteResponse>)
    )
)]
pub async fn list_credit_notes(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<CreditNoteResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    use obaf_database::entities::credit_note;
    use sea_orm::QueryOrder;

    let items = credit_note::Entity::find()
        .filter(credit_note::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .order_by_desc(credit_note::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    Ok(Json(items.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    get,
    path = "/api/v1/billing/credit-notes/{id}",
    tag = "Billing",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Credit note details", body = CreditNoteResponse),
        (status = 404, description = "Credit note not found", body = AuthErrorResponse)
    )
)]
pub async fn get_credit_note(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<CreditNoteResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    use obaf_database::entities::credit_note;

    let item = credit_note::Entity::find()
        .filter(credit_note::Column::Id.eq(id))
        .filter(credit_note::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Credit note not found".to_string(),
                }),
            )
        })?;

    Ok(Json(item.into()))
}

#[derive(Deserialize, ToSchema)]
pub struct SetExchangeRateRequest {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ExchangeRateResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<obaf_database::entities::currency_exchange_rate::Model> for ExchangeRateResponse {
    fn from(m: obaf_database::entities::currency_exchange_rate::Model) -> Self {
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            from_currency: m.from_currency,
            to_currency: m.to_currency,
            rate: m.rate,
            updated_at: m.updated_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/billing/exchange-rates",
    tag = "Billing",
    security(("bearer_auth" = [])),
    request_body = SetExchangeRateRequest,
    responses(
        (status = 200, description = "Exchange rate set successfully", body = ExchangeRateResponse),
        (status = 400, description = "Invalid exchange rate parameters", body = AuthErrorResponse)
    )
)]
pub async fn set_exchange_rate(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<SetExchangeRateRequest>,
) -> Result<Json<ExchangeRateResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    use obaf_database::entities::currency_exchange_rate;
    use sea_orm::{ActiveModelTrait, Set};

    let from = payload.from_currency.trim().to_uppercase();
    let to = payload.to_currency.trim().to_uppercase();

    if from.is_empty() || to.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Currency codes cannot be empty".to_string(),
            }),
        ));
    }

    if payload.rate <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Exchange rate must be greater than zero".to_string(),
            }),
        ));
    }

    let tenant_id = auth.tenant_id.as_uuid();
    let now = chrono::Utc::now();

    let existing = currency_exchange_rate::Entity::find()
        .filter(currency_exchange_rate::Column::TenantId.eq(tenant_id))
        .filter(currency_exchange_rate::Column::FromCurrency.eq(&from))
        .filter(currency_exchange_rate::Column::ToCurrency.eq(&to))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    let model = match existing {
        Some(item) => {
            let mut active: currency_exchange_rate::ActiveModel = item.into();
            active.rate = Set(payload.rate);
            active.updated_at = Set(now);
            active.update(&state.db).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AuthErrorResponse {
                        error: e.to_string(),
                    }),
                )
            })?
        }
        None => currency_exchange_rate::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            from_currency: Set(from),
            to_currency: Set(to),
            rate: Set(payload.rate),
            updated_at: Set(now),
        }
        .insert(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?,
    };

    Ok(Json(model.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/billing/exchange-rates",
    tag = "Billing",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of configured exchange rates", body = Vec<ExchangeRateResponse>)
    )
)]
pub async fn list_exchange_rates(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<ExchangeRateResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    use obaf_database::entities::currency_exchange_rate;

    let items = currency_exchange_rate::Entity::find()
        .filter(currency_exchange_rate::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(items.into_iter().map(Into::into).collect()))
}

#[derive(Deserialize, ToSchema)]
pub struct ConvertCurrencyRequest {
    pub amount: Decimal,
    pub from_currency: String,
    pub to_currency: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ConvertCurrencyResponse {
    pub original_amount: Decimal,
    pub from_currency: String,
    pub to_currency: String,
    pub exchange_rate: Decimal,
    pub converted_amount: Decimal,
}

#[utoipa::path(
    post,
    path = "/api/v1/billing/convert-currency",
    tag = "Billing",
    security(("bearer_auth" = [])),
    request_body = ConvertCurrencyRequest,
    responses(
        (status = 200, description = "Currency conversion result", body = ConvertCurrencyResponse),
        (status = 400, description = "Exchange rate not found or invalid", body = AuthErrorResponse)
    )
)]
pub async fn convert_currency(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<ConvertCurrencyRequest>,
) -> Result<Json<ConvertCurrencyResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    use obaf_database::entities::currency_exchange_rate;

    let from = payload.from_currency.trim().to_uppercase();
    let to = payload.to_currency.trim().to_uppercase();

    if from == to {
        return Ok(Json(ConvertCurrencyResponse {
            original_amount: payload.amount,
            from_currency: from,
            to_currency: to,
            exchange_rate: Decimal::ONE,
            converted_amount: payload.amount,
        }));
    }

    let tenant_id = auth.tenant_id.as_uuid();

    // Check direct rate
    let direct = currency_exchange_rate::Entity::find()
        .filter(currency_exchange_rate::Column::TenantId.eq(tenant_id))
        .filter(currency_exchange_rate::Column::FromCurrency.eq(&from))
        .filter(currency_exchange_rate::Column::ToCurrency.eq(&to))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    if let Some(r) = direct {
        let converted = (payload.amount * r.rate).round_dp(2);
        return Ok(Json(ConvertCurrencyResponse {
            original_amount: payload.amount,
            from_currency: from,
            to_currency: to,
            exchange_rate: r.rate,
            converted_amount: converted,
        }));
    }

    // Check inverse rate
    let inverse = currency_exchange_rate::Entity::find()
        .filter(currency_exchange_rate::Column::TenantId.eq(tenant_id))
        .filter(currency_exchange_rate::Column::FromCurrency.eq(&to))
        .filter(currency_exchange_rate::Column::ToCurrency.eq(&from))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    if let Some(r) = inverse {
        if r.rate > Decimal::ZERO {
            let inv_rate = (Decimal::ONE / r.rate).round_dp(6);
            let converted = (payload.amount / r.rate).round_dp(2);
            return Ok(Json(ConvertCurrencyResponse {
                original_amount: payload.amount,
                from_currency: from,
                to_currency: to,
                exchange_rate: inv_rate,
                converted_amount: converted,
            }));
        }
    }

    Err((
        StatusCode::BAD_REQUEST,
        Json(AuthErrorResponse {
            error: format!("No exchange rate configured between {} and {}", from, to),
        }),
    ))
}
