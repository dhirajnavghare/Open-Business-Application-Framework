pub mod audit;
pub mod error;
pub mod events;
pub mod extension;
pub mod money;
pub mod rules;
pub mod schema_validation;
pub mod security;
pub mod tenant;

pub use audit::AuditContext;
pub use error::{ObafError, Result};
pub use events::DomainEvent;
pub use extension::{ExtensionContext, ExtensionManifest, ExtensionRegistry, ObafExtension};
pub use money::{Currency, Money};
pub use rules::{
    evaluate_pricing_rules, PricingRule, RuleAction, RuleAdjustment, RuleCondition,
    RuleEvaluationResult,
};
pub use schema_validation::validate_metadata;
pub use security::{generate_token, hash_password, verify_password, verify_token, JwtClaims};
pub use tenant::{BranchId, TenantContext, TenantId};
