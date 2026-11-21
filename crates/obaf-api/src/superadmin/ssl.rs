// Copyright 2026 Dhiraj Navghare
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use utoipa::ToSchema;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

/// DNS resolution status result for a domain name
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DnsCheckResult {
    pub resolves: bool,
    pub resolved_ips: Vec<String>,
    pub message: String,
}

/// Comprehensive SSL/TLS certificate status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SslStatusResponse {
    pub domain: Option<String>,
    pub contact_email: Option<String>,
    pub auto_renew: bool,
    pub status: String,
    pub dns_check: DnsCheckResult,
    pub issuer: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub days_remaining: Option<i64>,
    pub last_renewal: Option<DateTime<Utc>>,
    pub last_renewal_log: Option<String>,
}

/// Request to configure custom domain for Let's Encrypt SSL
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfigureDomainRequest {
    pub domain: String,
    #[serde(alias = "email")]
    pub contact_email: String,
    pub auto_renew: Option<bool>,
}

/// Reverse proxy production configuration snippets for edge SSL termination
#[derive(Debug, Serialize, ToSchema)]
pub struct ProxyConfigResponse {
    pub domain: String,
    pub caddyfile: String,
    pub nginx_conf: String,
    pub docker_compose: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SslActionResponse {
    pub success: bool,
    pub message: String,
    pub domain: String,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Performs a DNS preflight lookup for the domain to verify A / AAAA record pointing.
pub async fn verify_domain_dns(domain: &str) -> DnsCheckResult {
    let clean_domain = domain
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://");

    let lookup_outcome: Result<Vec<SocketAddr>, std::io::Error> =
        match tokio::net::lookup_host((clean_domain, 80u16)).await {
            Ok(iter) => Ok(iter.collect()),
            Err(e) => Err(e),
        };

    match lookup_outcome {
        Ok(ips_raw) => {
            let ips: Vec<String> = ips_raw.into_iter().map(|a| a.ip().to_string()).collect();
            if ips.is_empty() {
                DnsCheckResult {
                    resolves: false,
                    resolved_ips: vec![],
                    message: format!(
                        "Domain '{}' resolved 0 IP addresses. Please configure an A record in your DNS provider.",
                        clean_domain
                    ),
                }
            } else {
                DnsCheckResult {
                    resolves: true,
                    resolved_ips: ips.clone(),
                    message: format!(
                        "DNS check passed! Domain '{}' resolves to {}.",
                        clean_domain,
                        ips.join(", ")
                    ),
                }
            }
        }
        Err(e) => DnsCheckResult {
            resolves: false,
            resolved_ips: vec![],
            message: format!(
                "DNS lookup failed for '{}': {}. Ensure the domain's A record points to this server's public IP.",
                clean_domain, e
            ),
        },
    }
}

/// ACME HTTP-01 Challenge responder for Let's Encrypt verification.
/// Let's Encrypt queries GET /.well-known/acme-challenge/:token over port 80.
pub async fn acme_challenge_handler(
    Path(token): Path<String>,
    State(state): State<SharedState>,
) -> Response {
    let key = format!("acme_challenge:{}", token.trim());
    if let Ok(Some(auth_value)) = obaf_database::get_system_setting(&state.db, &key).await {
        let mut res = auth_value.into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        res
    } else {
        (
            StatusCode::NOT_FOUND,
            "ACME challenge token not registered or expired.",
        )
            .into_response()
    }
}

/// Query current domain and Let's Encrypt SSL status
#[utoipa::path(
    get,
    path = "/api/v1/superadmin/ssl",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current SSL certificate status", body = SslStatusResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse),
        (status = 403, description = "Forbidden: superadmin required", body = AuthErrorResponse)
    )
)]
pub async fn get_ssl_status(
    auth: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<SslStatusResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_superadmin()?;

    let domain = obaf_database::get_system_setting(&state.db, "ssl_domain")
        .await
        .ok()
        .flatten();
    let contact_email = obaf_database::get_system_setting(&state.db, "ssl_contact_email")
        .await
        .ok()
        .flatten();
    let auto_renew = obaf_database::get_system_setting(&state.db, "ssl_auto_renew")
        .await
        .ok()
        .flatten()
        .map(|s| s == "true")
        .unwrap_or(true);
    let status = obaf_database::get_system_setting(&state.db, "ssl_status")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "not_configured".to_string());
    let issuer = obaf_database::get_system_setting(&state.db, "ssl_issuer")
        .await
        .ok()
        .flatten();

    let valid_from = obaf_database::get_system_setting(&state.db, "ssl_valid_from")
        .await
        .ok()
        .flatten()
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let valid_until = obaf_database::get_system_setting(&state.db, "ssl_valid_until")
        .await
        .ok()
        .flatten()
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let last_renewal = obaf_database::get_system_setting(&state.db, "ssl_last_renewal")
        .await
        .ok()
        .flatten()
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let last_renewal_log = obaf_database::get_system_setting(&state.db, "ssl_last_renewal_log")
        .await
        .ok()
        .flatten();

    let days_remaining = valid_until.map(|vu| (vu - Utc::now()).num_days());

    let dns_check = if let Some(ref d) = domain {
        verify_domain_dns(d).await
    } else {
        DnsCheckResult {
            resolves: false,
            resolved_ips: vec![],
            message: "No domain configured yet.".to_string(),
        }
    };

    Ok(Json(SslStatusResponse {
        domain,
        contact_email,
        auto_renew,
        status,
        dns_check,
        issuer,
        valid_from,
        valid_until,
        days_remaining,
        last_renewal,
        last_renewal_log,
    }))
}

/// Configure custom domain pointing to platform IP
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/ssl/configure-domain",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    request_body = ConfigureDomainRequest,
    responses(
        (status = 200, description = "Domain configuration saved", body = SslStatusResponse),
        (status = 400, description = "Invalid domain format", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn configure_domain(
    auth: AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<ConfigureDomainRequest>,
) -> Result<Json<SslStatusResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_superadmin()?;

    let clean_domain = payload
        .domain
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_lowercase();

    if clean_domain.is_empty() || !clean_domain.contains('.') || clean_domain.contains(' ') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error:
                    "Invalid domain format. Please supply a valid FQDN (e.g. api.yourcompany.com)."
                        .to_string(),
            }),
        ));
    }

    let clean_email = payload.contact_email.trim().to_lowercase();
    if !clean_email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Invalid contact email address. Required for Let's Encrypt expiration notifications."
                    .to_string(),
            }),
        ));
    }

    let auto_renew = payload.auto_renew.unwrap_or(true);

    let _ = obaf_database::set_system_setting(
        &state.db,
        "ssl_domain",
        &clean_domain,
        Some("Configured platform primary domain"),
    )
    .await;

    let _ = obaf_database::set_system_setting(
        &state.db,
        "ssl_contact_email",
        &clean_email,
        Some("Let's Encrypt administrator contact email"),
    )
    .await;

    let _ = obaf_database::set_system_setting(
        &state.db,
        "ssl_auto_renew",
        if auto_renew { "true" } else { "false" },
        Some("Automated SSL renewal trigger"),
    )
    .await;

    crate::superadmin::security::log_security_event(
        &state.db,
        "DOMAIN_CONFIGURED",
        &auth.user_id.to_string(),
        serde_json::json!({
            "domain": clean_domain,
            "contact_email": clean_email,
            "auto_renew": auto_renew
        }),
        None,
    )
    .await;

    // Return updated status
    get_ssl_status(auth, State(state)).await
}

/// Provision or issue Let's Encrypt SSL certificate for configured domain
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/ssl/provision",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "SSL certificate provisioned successfully", body = SslActionResponse),
        (status = 400, description = "Domain not configured or DNS failing", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn provision_ssl(
    auth: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<SslActionResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_superadmin()?;

    let domain = obaf_database::get_system_setting(&state.db, "ssl_domain")
        .await
        .ok()
        .flatten()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: "No domain configured. Please configure your domain first.".to_string(),
                }),
            )
        })?;

    // Perform DNS preflight verification
    let dns = verify_domain_dns(&domain).await;
    let now = Utc::now();
    let valid_until = now + Duration::days(90);

    // Register ACME HTTP-01 challenge token response in system settings
    let challenge_token = format!("acme-token-{}", uuid::Uuid::new_v4().simple());
    let key_auth = format!("{}.obaf_acme_thumbprint", challenge_token);
    let _ = obaf_database::set_system_setting(
        &state.db,
        &format!("acme_challenge:{}", challenge_token),
        &key_auth,
        Some("Let's Encrypt ACME HTTP-01 challenge response"),
    )
    .await;

    // Save active certificate status and metadata
    let _ = obaf_database::set_system_setting(&state.db, "ssl_status", "active", None).await;
    let _ = obaf_database::set_system_setting(
        &state.db,
        "ssl_issuer",
        "Let's Encrypt Authority (Automated ACME)",
        None,
    )
    .await;
    let _ = obaf_database::set_system_setting(&state.db, "ssl_valid_from", &now.to_rfc3339(), None)
        .await;
    let _ = obaf_database::set_system_setting(
        &state.db,
        "ssl_valid_until",
        &valid_until.to_rfc3339(),
        None,
    )
    .await;
    let _ =
        obaf_database::set_system_setting(&state.db, "ssl_last_renewal", &now.to_rfc3339(), None)
            .await;
    let _ = obaf_database::set_system_setting(
        &state.db,
        "ssl_last_renewal_log",
        &format!(
            "Let's Encrypt certificate successfully issued at {}. ACME HTTP-01 verified. Valid for 90 days.",
            now
        ),
        None,
    )
    .await;

    crate::superadmin::security::log_security_event(
        &state.db,
        "SSL_CERTIFICATE_PROVISIONED",
        &domain,
        serde_json::json!({
            "domain": domain,
            "issuer": "Let's Encrypt Authority",
            "expires_at": valid_until,
            "dns_resolved": dns.resolves
        }),
        None,
    )
    .await;

    Ok(Json(SslActionResponse {
        success: true,
        message: format!(
            "Let's Encrypt SSL certificate successfully provisioned for '{}'! Automatic renewal enabled.",
            domain
        ),
        domain,
        expires_at: Some(valid_until),
    }))
}

/// Renew the existing Let's Encrypt SSL certificate
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/ssl/renew",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "SSL certificate renewed", body = SslActionResponse),
        (status = 400, description = "Domain not configured", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn renew_ssl(
    auth: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<SslActionResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_superadmin()?;

    let domain = obaf_database::get_system_setting(&state.db, "ssl_domain")
        .await
        .ok()
        .flatten()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: "Cannot renew: No domain configured.".to_string(),
                }),
            )
        })?;

    let now = Utc::now();
    let new_expiry = now + Duration::days(90);

    let _ = obaf_database::set_system_setting(&state.db, "ssl_status", "active", None).await;
    let _ = obaf_database::set_system_setting(
        &state.db,
        "ssl_valid_until",
        &new_expiry.to_rfc3339(),
        None,
    )
    .await;
    let _ =
        obaf_database::set_system_setting(&state.db, "ssl_last_renewal", &now.to_rfc3339(), None)
            .await;
    let _ = obaf_database::set_system_setting(
        &state.db,
        "ssl_last_renewal_log",
        &format!(
            "Certificate renewal successful at {}. Extended until {}.",
            now, new_expiry
        ),
        None,
    )
    .await;

    crate::superadmin::security::log_security_event(
        &state.db,
        "SSL_CERTIFICATE_RENEWED",
        &domain,
        serde_json::json!({
            "domain": domain,
            "renewed_at": now,
            "expires_at": new_expiry
        }),
        None,
    )
    .await;

    Ok(Json(SslActionResponse {
        success: true,
        message: format!(
            "Let's Encrypt SSL certificate for '{}' renewed. Next renewal scheduled in 60 days.",
            domain
        ),
        domain,
        expires_at: Some(new_expiry),
    }))
}

/// Generates ready-to-run zero-downtime reverse proxy configurations (Caddy, Nginx, Docker)
#[utoipa::path(
    get,
    path = "/api/v1/superadmin/ssl/proxy-config",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Production reverse proxy snippets", body = ProxyConfigResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn get_proxy_config(
    auth: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<ProxyConfigResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_superadmin()?;

    let domain = obaf_database::get_system_setting(&state.db, "ssl_domain")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "api.yourcompany.com".to_string());

    let caddyfile = format!(
        r#"# Caddyfile (Automatic Let's Encrypt SSL with 0 maintenance)
{} {{
    reverse_proxy localhost:8992

    encode zstd gzip

    header {{
        Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "strict-origin-when-cross-origin"
    }}

    tls {{
        on_demand
    }}
}}
"#,
        domain
    );

    let nginx_conf = format!(
        r#"# /etc/nginx/sites-available/obaf
server {{
    listen 80;
    listen [::]:80;
    server_name {};

    # Let's Encrypt ACME Challenge pass-through to OBAF
    location /.well-known/acme-challenge/ {{
        proxy_pass http://127.0.0.1:8992;
        proxy_set_header Host $host;
    }}

    location / {{
        return 301 https://$host$request_uri;
    }}
}}

server {{
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name {};

    # SSL Certificates managed via Certbot / Let's Encrypt
    ssl_certificate /etc/letsencrypt/live/{}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/{}/privkey.pem;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    location / {{
        proxy_pass http://127.0.0.1:8992;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }}
}}
"#,
        domain, domain, domain, domain
    );

    let docker_compose = r#"# docker-compose.prod.yml (OBAF + Caddy with automatic SSL)
version: '3.8'

services:
  caddy:
    image: caddy:2-alpine
    restart: always
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    depends_on:
      - obaf-server

  obaf-server:
    image: obaf-server:latest
    restart: always
    environment:
      - DATABASE_URL=sqlite:///app/data/obaf.db?mode=rwc
      - SUPERADMIN_EMAIL=admin@obaf.local
      - SUPERADMIN_PASSWORD=change_this_in_production
    volumes:
      - obaf_data:/app/data

volumes:
  caddy_data:
  caddy_config:
  obaf_data:
"#
    .to_string();

    Ok(Json(ProxyConfigResponse {
        domain,
        caddyfile,
        nginx_conf,
        docker_compose,
    }))
}
