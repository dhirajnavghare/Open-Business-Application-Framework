use obaf_core::{
    DomainEvent, ExtensionContext, ExtensionManifest, ExtensionRegistry, ObafExtension, Result,
    TenantId,
};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct MockAuditExtension {
    events_received: Arc<AtomicUsize>,
}

impl ObafExtension for MockAuditExtension {
    fn manifest(&self) -> ExtensionManifest {
        ExtensionManifest {
            name: "mock-audit-extension".into(),
            version: "0.1.0".into(),
            description: "Verifies module contract and event delivery".into(),
            author: "Core Test Team".into(),
            required_modules: vec!["orders".into()],
        }
    }

    fn on_init(&self, _ctx: &ExtensionContext) -> Result<()> {
        Ok(())
    }

    fn on_event(&self, event: &DomainEvent<serde_json::Value>) -> Result<()> {
        if event.event_type.starts_with("order.") {
            self.events_received.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_extension_contract_lifecycle_and_dispatch() {
    let registry = ExtensionRegistry::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let ext = MockAuditExtension {
        events_received: counter.clone(),
    };

    // Register extension
    assert!(registry.register(ext).is_ok());

    // Verify manifest discovery
    let manifests = registry.list_manifests();
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].name, "mock-audit-extension");
    assert_eq!(manifests[0].required_modules, vec!["orders"]);

    // Dispatch matching domain event
    let event = DomainEvent::new(
        "order.created",
        TenantId::new(),
        json!({
            "order_number": "ORD-CONTRACT-999",
            "total": 150.0
        }),
    );

    assert!(registry.dispatch_event(&event).is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Dispatch non-matching event
    let non_matching = DomainEvent::new(
        "customer.created",
        TenantId::new(),
        json!({ "name": "Jane" }),
    );
    assert!(registry.dispatch_event(&non_matching).is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}
