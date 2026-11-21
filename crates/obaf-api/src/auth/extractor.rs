use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    Json,
};
use chrono::Utc;
use obaf_core::{verify_token, TenantId};
use obaf_database::entities::api_key;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::SharedState;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub tenant_id: TenantId,
    pub role: String,
}

#[derive(Serialize, ToSchema)]
pub struct AuthErrorResponse {
    pub error: String,
}

impl AuthUser {
    pub fn is_superadmin(&self) -> bool {
        self.role == "superadmin"
    }

    pub fn require_superadmin(&self) -> Result<(), (StatusCode, Json<AuthErrorResponse>)> {
        if self.is_superadmin() {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                Json(AuthErrorResponse {
                    error: "Forbidden: Platform superadmin privileges required".to_string(),
                }),
            ))
        }
    }

    pub fn require_role(
        &self,
        allowed_roles: &[&str],
    ) -> Result<(), (StatusCode, Json<AuthErrorResponse>)> {
        if self.is_superadmin() || allowed_roles.contains(&self.role.as_str()) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                Json(AuthErrorResponse {
                    error: format!(
                        "Forbidden: Role '{}' does not have sufficient permissions. Required one of: {:?}",
                        self.role, allowed_roles
                    ),
                }),
            ))
        }
    }
}

#[async_trait]
impl FromRequestParts<SharedState> for AuthUser {
    type Rejection = (StatusCode, Json<AuthErrorResponse>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        // 1. Check for X-API-Key header (Machine-to-Machine Service Auth)
        if let Some(api_key_header) = parts
            .headers
            .get("x-api-key")
            .and_then(|val| val.to_str().ok())
        {
            let mut hasher = Sha256::new();
            hasher.update(api_key_header.trim().as_bytes());
            let hash = hex::encode(hasher.finalize());

            let found = api_key::Entity::find()
                .filter(api_key::Column::KeyHash.eq(hash))
                .filter(api_key::Column::IsActive.eq(true))
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

            if let Some(k) = found {
                // Check expiration if set
                if let Some(exp) = k.expires_at {
                    if exp < Utc::now() {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(AuthErrorResponse {
                                error: "API Key has expired".to_string(),
                            }),
                        ));
                    }
                }

                return Ok(AuthUser {
                    user_id: k.id,
                    tenant_id: TenantId(k.tenant_id),
                    role: "admin".to_string(), // API keys possess administrative machine scope
                });
            } else {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(AuthErrorResponse {
                        error: "Invalid or inactive API Key".to_string(),
                    }),
                ));
            }
        }

        // 2. Fall back to Bearer JWT Token
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|val| val.to_str().ok());

        let token = match auth_header {
            Some(header) if header.starts_with("Bearer ") => &header[7..],
            _ => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(AuthErrorResponse {
                        error: "Missing or invalid authentication. Provide 'Authorization: Bearer <token>' or 'X-API-Key: <key>' header".to_string(),
                    }),
                ));
            }
        };

        let claims = verify_token(token, &state.jwt_secret).map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthErrorResponse {
                    error: format!("Unauthorized: {}", e),
                }),
            )
        })?;

        // Intermediate MFA challenge tokens are strictly prohibited from authenticating standard APIs
        if claims.role == "superadmin_mfa_pending" {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(AuthErrorResponse {
                    error: "MFA challenge pending: please complete TOTP 2FA verification before accessing platform endpoints."
                        .to_string(),
                }),
            ));
        }

        Ok(AuthUser {
            user_id: claims.sub,
            tenant_id: claims.tenant_id,
            role: claims.role,
        })
    }
}
