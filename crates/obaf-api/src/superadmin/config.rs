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

use std::env;

/// Configuration for the single top-level Platform Super Administrator.
/// Loaded exclusively from environment variables.
#[derive(Clone, Debug)]
pub struct SuperAdminConfig {
    /// Super Admin login email address
    pub email: String,
    /// Super Admin login password (plain text or Argon2 password hash)
    pub password: String,
    /// Optional Google OAuth 2.0 Client ID for Google SSO
    pub google_client_id: Option<String>,
    /// Optional Google OAuth 2.0 Client Secret
    pub google_client_secret: Option<String>,
    /// Authorized Google account email (defaults to `email` if not explicitly set)
    pub google_email: String,
}

impl SuperAdminConfig {
    /// Load Super Admin configuration from environment variables with safe defaults.
    pub fn from_env() -> Self {
        let email = env::var("OBAF_SUPERADMIN_EMAIL")
            .or_else(|_| env::var("SUPERADMIN_EMAIL"))
            .unwrap_or_else(|_| "admin@obaf.local".to_string())
            .trim()
            .to_lowercase();

        let password = env::var("OBAF_SUPERADMIN_PASSWORD")
            .or_else(|_| env::var("SUPERADMIN_PASSWORD"))
            .unwrap_or_else(|_| "admin123".to_string());

        let google_client_id = env::var("OBAF_GOOGLE_CLIENT_ID")
            .or_else(|_| env::var("GOOGLE_CLIENT_ID"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let google_client_secret = env::var("OBAF_GOOGLE_CLIENT_SECRET")
            .or_else(|_| env::var("GOOGLE_CLIENT_SECRET"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let google_email = env::var("OBAF_SUPERADMIN_GOOGLE_EMAIL")
            .or_else(|_| env::var("SUPERADMIN_GOOGLE_EMAIL"))
            .unwrap_or_else(|_| email.clone())
            .trim()
            .to_lowercase();

        Self {
            email,
            password,
            google_client_id,
            google_client_secret,
            google_email,
        }
    }

    /// Verifies the provided password against configured password.
    /// Supports both raw plain-text passwords and Argon2 password hashes.
    pub fn verify_password(&self, input: &str) -> bool {
        if self.password.starts_with("$argon2") {
            if let Ok(valid) = obaf_core::security::verify_password(input, &self.password) {
                return valid;
            }
        }
        constant_time_eq(input.as_bytes(), self.password.as_bytes())
    }

    /// Verifies whether the given email matches the designated Super Admin Google email.
    pub fn verify_google_email(&self, email: &str) -> bool {
        let cleaned = email.trim().to_lowercase();
        cleaned == self.google_email || cleaned == self.email
    }

    /// Whether Google SSO authentication is enabled and configured.
    pub fn is_google_auth_enabled(&self) -> bool {
        self.google_client_id.is_some()
    }

    /// Whether Super Admin is using the default out-of-the-box password ("admin123")
    pub fn is_default_password(&self) -> bool {
        self.password == "admin123"
    }
}

impl Default for SuperAdminConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Constant-time byte slice comparison to mitigate timing side-channel attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
