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
        .checked_add_signed(Duration::hours(expiry_hours.min(87600) as i64))
        .ok_or_else(|| ObafError::validation("Token expiry calculation out of bounds"))?
        .timestamp() as usize;

    let claims = JwtClaims {
        sub: user_id,
        tenant_id,
        role: role.to_string(),
        exp: expiration,
    };

    let mut header = Header::new(jsonwebtoken::Algorithm::HS256);
    header.typ = Some("JWT".to_string());

    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ObafError::internal(format!("JWT generation failed: {}", e)))
}

pub fn verify_token(token: &str, secret: &str) -> Result<JwtClaims> {
    // Strictly lock algorithm to HS256 to eliminate Algorithm Confusion and none-alg attacks
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;
    validation.leeway = 60;

    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| ObafError::unauthorized(format!("Invalid or expired token: {}", e)))?;

    Ok(token_data.claims)
}

/// Constant-time string equality comparison to prevent side-channel timing attacks.
pub fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let password = Uuid::new_v4().to_string();
        let hash = hash_password(&password).unwrap();
        assert!(hash.starts_with("$argon2"));

        // Correct password verifies successfully
        assert!(verify_password(&password, &hash).unwrap());

        // Incorrect password fails verification
        let wrong_password = format!("{password}_wrong");
        assert!(!verify_password(&wrong_password, &hash).unwrap());

        // Malformed hash returns error
        assert!(verify_password(&password, "invalid_hash").is_err());
    }

    #[test]
    fn test_jwt_token_roundtrip_and_bounds() {
        let user_id = Uuid::new_v4();
        let tenant_id = TenantId(Uuid::new_v4());
        let secret = "jwt_signing_secret_for_unit_tests_123456789";

        let token = generate_token(user_id, tenant_id, "admin", secret, 24).unwrap();
        let claims = verify_token(&token, secret).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.tenant_id, tenant_id);
        assert_eq!(claims.role, "admin");

        // Wrong secret fails validation
        assert!(verify_token(&token, "wrong_secret_123").is_err());

        // Excessive expiry hours clamped safely without panicking
        let clamped_token = generate_token(user_id, tenant_id, "admin", secret, u64::MAX);
        assert!(clamped_token.is_ok());
    }

    #[test]
    fn test_constant_time_comparison() {
        assert!(constant_time_compare("abc123XYZ", "abc123XYZ"));
        assert!(!constant_time_compare("abc123XYZ", "abc123xyZ"));
        assert!(!constant_time_compare("abc", "abcdef"));
        assert!(!constant_time_compare("", "a"));
        assert!(constant_time_compare("", ""));
    }
}
