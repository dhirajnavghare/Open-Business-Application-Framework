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

pub mod audit;
pub mod cache;
pub mod circuit_breaker;
pub mod error;
pub mod events;
pub mod extension;
pub mod money;
pub mod rate_limiter;
pub mod rules;
pub mod schema_validation;
pub mod security;
pub mod tenant;
pub mod totp;

pub use audit::AuditContext;
pub use cache::TtlCache;
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerError, CircuitState};
pub use error::{ObafError, Result};
pub use events::DomainEvent;
pub use extension::{ExtensionContext, ExtensionManifest, ExtensionRegistry, ObafExtension};
pub use money::{Currency, Money};
pub use rate_limiter::{RateLimitResult, TokenBucketRateLimiter};
pub use rules::{
    evaluate_pricing_rules, PricingRule, RuleAction, RuleAdjustment, RuleCondition,
    RuleEvaluationResult,
};
pub use schema_validation::validate_metadata;
pub use security::{generate_token, hash_password, verify_password, verify_token, JwtClaims};
pub use tenant::{BranchId, TenantContext, TenantId};
pub use totp::*;

#[cfg(test)]
mod compile_time_tests {
    use super::*;
    use static_assertions::assert_impl_all;

    // Compile-time static assertions ensuring thread-safety & trait invariants
    assert_impl_all!(Money: Send, Sync, Clone, PartialEq);
    assert_impl_all!(AuditContext: Send, Sync, Clone);
    assert_impl_all!(TenantContext: Send, Sync, Clone);
    assert_impl_all!(TokenBucketRateLimiter: Send, Sync);
    assert_impl_all!(CircuitBreaker: Send, Sync);
    assert_impl_all!(TtlCache<String, String>: Send, Sync);
    assert_impl_all!(ExtensionRegistry: Send, Sync);
}
