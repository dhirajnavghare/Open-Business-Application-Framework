//! Example Community Extension: Loyalty Rewards
//! Demonstrates how an open-source contributor can extend OBAF by implementing `ObafExtension`.

use obaf_core::{DomainEvent, ExtensionContext, ExtensionManifest, ObafExtension, Result};
use tracing::info;

pub struct LoyaltyRewardsExtension {
    points_multiplier: u32,
}

impl LoyaltyRewardsExtension {
    pub fn new(points_multiplier: u32) -> Self {
        Self { points_multiplier }
    }
}

impl Default for LoyaltyRewardsExtension {
    fn default() -> Self {
        Self::new(10) // 10 points per dollar spent
    }
}

impl ObafExtension for LoyaltyRewardsExtension {
    fn manifest(&self) -> ExtensionManifest {
        ExtensionManifest {
            name: "loyalty-rewards".to_string(),
            version: "1.0.0".to_string(),
            description: "Calculates customer reward points on order completion".to_string(),
            author: "Community Contributor <dev@example.com>".to_string(),
            required_modules: vec!["orders".to_string(), "customers".to_string()],
        }
    }

    fn on_init(&self, _ctx: &ExtensionContext) -> Result<()> {
        info!("LoyaltyRewardsExtension initialized successfully");
        Ok(())
    }

    fn on_event(&self, event: &DomainEvent<serde_json::Value>) -> Result<()> {
        if event.event_type == "order.completed" {
            if let Some(total) = event.payload.get("grand_total").and_then(|v| v.as_f64()) {
                let points = (total * self.points_multiplier as f64).round() as u64;
                info!(
                    tenant_id = %event.tenant_id,
                    order_id = %event.id,
                    reward_points = points,
                    "Loyalty points credited to customer account"
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obaf_core::TenantId;
    use serde_json::json;

    #[test]
    fn test_loyalty_extension_contract() {
        let ext = LoyaltyRewardsExtension::default();
        let manifest = ext.manifest();
        assert_eq!(manifest.name, "loyalty-rewards");
        assert_eq!(manifest.required_modules, vec!["orders", "customers"]);

        let event = DomainEvent::new(
            "order.completed",
            TenantId::new(),
            json!({
                "grand_total": 50.0
            }),
        );

        assert!(ext.on_event(&event).is_ok());
    }
}
