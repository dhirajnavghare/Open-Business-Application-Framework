use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    Json,
};
use obaf_core::{verify_token, TenantId};
use serde::Serialize;
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
    pub fn require_role(
        &self,
        allowed_roles: &[&str],
    ) -> Result<(), (StatusCode, Json<AuthErrorResponse>)> {
        if allowed_roles.contains(&self.role.as_str()) {
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
                        error: "Missing or invalid 'Authorization: Bearer <token>' header"
                            .to_string(),
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

        Ok(AuthUser {
            user_id: claims.sub,
            tenant_id: claims.tenant_id,
            role: claims.role,
        })
    }
}
