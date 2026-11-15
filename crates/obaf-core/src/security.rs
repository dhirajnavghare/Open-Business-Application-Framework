use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ObafError, Result};
use crate::tenant::TenantId;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JwtClaims {
    pub sub: Uuid,           // user_id
    pub tenant_id: TenantId, // organization_id
    pub role: String,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ObafError::internal(format!("Password hashing failed: {}", e)))?
        .to_string();

    Ok(password_hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| ObafError::internal(format!("Invalid password hash format: {}", e)))?;

    let is_valid = Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(is_valid)
}

pub fn generate_token(
    user_id: Uuid,
    tenant_id: TenantId,
    role: &str,
    secret: &str,
    expiry_hours: u64,
) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(expiry_hours as i64))
        .expect("Valid timestamp")
        .timestamp() as usize;

    let claims = JwtClaims {
        sub: user_id,
        tenant_id,
        role: role.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ObafError::internal(format!("JWT generation failed: {}", e)))
}

pub fn verify_token(token: &str, secret: &str) -> Result<JwtClaims> {
    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| ObafError::unauthorized(format!("Invalid or expired token: {}", e)))?;

    Ok(token_data.claims)
}
