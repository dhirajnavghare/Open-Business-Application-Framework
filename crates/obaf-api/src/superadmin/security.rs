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
use obaf_database::entities::audit_log;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Maximum consecutive failed login attempts before lockout
pub const MAX_FAILED_LOGIN_ATTEMPTS: u32 = 5;
/// Duration of lockout window in seconds (15 minutes)
pub const LOCKOUT_DURATION_SECS: u64 = 900;

#[derive(Clone, Debug)]
struct AttemptState {
    count: u32,
    first_attempt: Instant,
    locked_until: Option<Instant>,
}

/// In-memory thread-safe guard against brute-force attacks and credential stuffing
#[derive(Clone, Default)]
pub struct LoginAttemptGuard {
    attempts: Arc<RwLock<HashMap<String, AttemptState>>>,
}

impl LoginAttemptGuard {
    pub fn new() -> Self {
        Self {
            attempts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if the identifier (IP or username) is currently locked out.
    /// Returns `Err(retry_after_secs)` if locked out, or `Ok(())` if allowed.
    pub fn check_lockout(&self, key: &str) -> Result<(), u64> {
        let now = Instant::now();
        let map = self.attempts.read().unwrap_or_else(|e| e.into_inner());

        if let Some(state) = map.get(key) {
            if let Some(locked_until) = state.locked_until {
                if now < locked_until {
                    let remaining = locked_until.duration_since(now).as_secs();
                    return Err(remaining.max(1));
                }
            }
        }

        Ok(())
    }

    /// Records a failed authentication attempt.
    /// If failure count reaches `MAX_FAILED_LOGIN_ATTEMPTS`, initiates a progressive lockout.
    pub fn record_failure(&self, key: &str) -> (u32, bool) {
        let now = Instant::now();
        let mut map = self.attempts.write().unwrap_or_else(|e| e.into_inner());

        let state = map.entry(key.to_string()).or_insert_with(|| AttemptState {
            count: 0,
            first_attempt: now,
            locked_until: None,
        });

        // Reset if window expired
        if now.duration_since(state.first_attempt) > Duration::from_secs(LOCKOUT_DURATION_SECS)
            && state.locked_until.is_none()
        {
            state.count = 0;
            state.first_attempt = now;
        }

        state.count += 1;
        let is_locked = if state.count >= MAX_FAILED_LOGIN_ATTEMPTS {
            state.locked_until = Some(now + Duration::from_secs(LOCKOUT_DURATION_SECS));
            true
        } else {
            false
        };

        (state.count, is_locked)
    }

    /// Resets the failure counter upon successful authentication.
    pub fn record_success(&self, key: &str) {
        let mut map = self.attempts.write().unwrap_or_else(|e| e.into_inner());
        map.remove(key);
    }
}

/// Verifies whether a given client IP address is permitted according to an allowed IPs configuration.
/// Supports both single IP addresses and CIDR notation (e.g. `192.168.1.100`, `10.0.0.0/8`, `2001:db8::/32`).
/// If `allowed_config` is empty, returns `true` (unrestricted).
pub fn is_ip_allowed(client_ip_str: &str, allowed_config: &str) -> bool {
    let trimmed_cfg = allowed_config.trim();
    if trimmed_cfg.is_empty() {
        return true;
    }

    let Ok(client_ip) = IpAddr::from_str(client_ip_str.trim()) else {
        return false;
    };

    for raw_entry in trimmed_cfg.split(',') {
        let entry = raw_entry.trim();
        if entry.is_empty() {
            continue;
        }

        if let Some((cidr_ip, prefix_str)) = entry.split_once('/') {
            if let Ok(prefix) = prefix_str.parse::<u8>() {
                if cidr_matches(&client_ip, cidr_ip.trim(), prefix) {
                    return true;
                }
            }
        } else if let Ok(allowed_ip) = IpAddr::from_str(entry) {
            if client_ip == allowed_ip {
                return true;
            }
        }
    }

    false
}

fn cidr_matches(client: &IpAddr, network_str: &str, prefix: u8) -> bool {
    match (client, IpAddr::from_str(network_str)) {
        (IpAddr::V4(c_v4), Ok(IpAddr::V4(n_v4))) => {
            if prefix > 32 {
                return false;
            }
            if prefix == 0 {
                return true;
            }
            let mask = !((1u32 << (32 - prefix)) - 1);
            (u32::from(*c_v4) & mask) == (u32::from(n_v4) & mask)
        }
        (IpAddr::V6(c_v6), Ok(IpAddr::V6(n_v6))) => {
            if prefix > 128 {
                return false;
            }
            if prefix == 0 {
                return true;
            }
            let c_bytes = c_v6.octets();
            let n_bytes = n_v6.octets();
            let full_bytes = (prefix / 8) as usize;
            let remainder_bits = prefix % 8;

            if c_bytes[..full_bytes] != n_bytes[..full_bytes] {
                return false;
            }

            if remainder_bits > 0 {
                let mask = !((1u8 << (8 - remainder_bits)) - 1);
                if (c_bytes[full_bytes] & mask) != (n_bytes[full_bytes] & mask) {
                    return false;
                }
            }

            true
        }
        _ => false,
    }
}

/// Logs a high-security platform audit event into the immutable database audit trail.
pub async fn log_security_event(
    db: &DatabaseConnection,
    action: &str,
    actor_email: &str,
    details: serde_json::Value,
    ip_address: Option<&str>,
) {
    let model = audit_log::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(Uuid::nil()), // Platform root tenant
        actor_id: Set(Some(crate::superadmin::auth::SUPERADMIN_USER_ID)),
        action: Set(action.to_string()),
        entity_type: Set("platform_security".to_string()),
        entity_id: Set(Some(actor_email.to_string())),
        details: Set(details.to_string()),
        ip_address: Set(ip_address.map(|s| s.to_string())),
        created_at: Set(Utc::now()),
    };

    if let Err(e) = model.insert(db).await {
        tracing::error!("Failed to record security audit log: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_whitelisting_single_ips() {
        let config = "192.168.1.100, 10.0.0.5, 127.0.0.1";
        assert!(is_ip_allowed("192.168.1.100", config));
        assert!(is_ip_allowed("127.0.0.1", config));
        assert!(is_ip_allowed("10.0.0.5", config));
        assert!(!is_ip_allowed("192.168.1.101", config));
        assert!(!is_ip_allowed("8.8.8.8", config));
    }

    #[test]
    fn test_ip_whitelisting_cidr() {
        let config = "10.0.0.0/8, 192.168.1.0/24, 2001:db8::/32";
        assert!(is_ip_allowed("10.254.1.99", config));
        assert!(is_ip_allowed("192.168.1.42", config));
        assert!(is_ip_allowed("2001:db8:0:1::42", config));
        assert!(!is_ip_allowed("192.168.2.1", config));
        assert!(!is_ip_allowed("11.0.0.1", config));
        assert!(!is_ip_allowed("2001:db9::1", config));
    }

    #[test]
    fn test_ip_whitelisting_empty_is_unrestricted() {
        assert!(is_ip_allowed("1.2.3.4", ""));
        assert!(is_ip_allowed("127.0.0.1", "   "));
    }

    #[test]
    fn test_login_attempt_guard_lockout() {
        let guard = LoginAttemptGuard::new();
        let key = "admin@obaf.local:192.168.1.50";

        assert!(guard.check_lockout(key).is_ok());

        // 4 failed attempts: not locked out yet
        for i in 1..=4 {
            let (count, is_locked) = guard.record_failure(key);
            assert_eq!(count, i);
            assert!(!is_locked);
            assert!(guard.check_lockout(key).is_ok());
        }

        // 5th failed attempt triggers lockout
        let (count, is_locked) = guard.record_failure(key);
        assert_eq!(count, 5);
        assert!(is_locked);
        assert!(guard.check_lockout(key).is_err());

        // Success resets counter
        guard.record_success(key);
        assert!(guard.check_lockout(key).is_ok());
    }
}
