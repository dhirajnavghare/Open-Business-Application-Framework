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

use chrono::Utc;
use obaf_core::{ObafError, Result};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;

use crate::entities::system_setting;

/// System setting keys
pub const JWT_SECRET_KEY: &str = "jwt_secret";
pub const PLATFORM_NAME_KEY: &str = "platform_name";
pub const DEFAULT_CURRENCY_KEY: &str = "default_currency";
pub const TOKEN_EXPIRY_HOURS_KEY: &str = "token_expiry_hours";
pub const GOOGLE_CLIENT_ID_KEY: &str = "google_client_id";

/// Retrieve existing system setting value by key
pub async fn get_system_setting(db: &DatabaseConnection, key: &str) -> Result<Option<String>> {
    let setting = system_setting::Entity::find_by_id(key.to_string())
        .one(db)
        .await
        .map_err(|e| ObafError::Database {
            message: format!("Failed to query system setting '{}': {}", key, e),
        })?;
    Ok(setting.map(|s| s.value))
}

/// Set or update a system setting value
pub async fn set_system_setting(
    db: &DatabaseConnection,
    key: &str,
    value: &str,
    description: Option<&str>,
) -> Result<()> {
    let existing = system_setting::Entity::find_by_id(key.to_string())
        .one(db)
        .await
        .map_err(|e| ObafError::Database {
            message: format!("Failed to read system setting '{}': {}", key, e),
        })?;

    if let Some(model) = existing {
        let mut active: system_setting::ActiveModel = model.into();
        active.value = Set(value.to_string());
        if let Some(desc) = description {
            active.description = Set(Some(desc.to_string()));
        }
        active.updated_at = Set(Utc::now());
        active.update(db).await.map_err(|e| ObafError::Database {
            message: format!("Failed to update system setting '{}': {}", key, e),
        })?;
    } else {
        let active = system_setting::ActiveModel {
            key: Set(key.to_string()),
            value: Set(value.to_string()),
            description: Set(description.map(|d| d.to_string())),
            updated_at: Set(Utc::now()),
        };
        active.insert(db).await.map_err(|e| ObafError::Database {
            message: format!("Failed to insert system setting '{}': {}", key, e),
        })?;
    }

    Ok(())
}

/// Retrieves the persistent JWT secret from database, or generates a high-entropy 64-character
/// secret on first boot and saves it to system_settings.
pub async fn get_or_create_jwt_secret(db: &DatabaseConnection) -> Result<String> {
    if let Some(existing) = get_system_setting(db, JWT_SECRET_KEY).await? {
        if !existing.trim().is_empty() {
            return Ok(existing);
        }
    }

    // Generate high-entropy 64-character secret from cryptographic UUID sources
    let secret = format!(
        "{}{}{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let secret_64 = &secret[..64];

    set_system_setting(
        db,
        JWT_SECRET_KEY,
        secret_64,
        Some("Auto-generated persistent cryptographic JWT signing secret"),
    )
    .await?;

    Ok(secret_64.to_string())
}
