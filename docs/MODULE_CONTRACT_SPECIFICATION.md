# OBAF Module & Extension Contract Specification

## 1. Extension Model Overview

OBAF separates generic business capabilities from domain-specific rules using two mechanisms:
1. **Declarative Domain Presets** (JSON configuration bootstrapping active modules, tax slabs, and custom schemas).
2. **Compiled Rust Extensions** (`ObafExtension` trait) for deep lifecycle hooks and domain event handling.

---

## 2. The `ObafExtension` Trait

Third-party contributors and custom domain developers implement the `ObafExtension` trait in `obaf-core`:

```rust
use obaf_core::{DomainEvent, ExtensionContext, ExtensionManifest, Result};

pub trait ObafExtension: Send + Sync {
    /// Returns static extension metadata
    fn manifest(&self) -> ExtensionManifest;

    /// Called during application startup
    fn on_init(&self, ctx: &ExtensionContext) -> Result<()>;

    /// Called whenever domain events are published
    fn on_event(&self, event: &DomainEvent<serde_json::Value>) -> Result<()>;
}
```

### 2.1 Extension Manifest Schema
```rust
pub struct ExtensionManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub required_modules: Vec<String>,
}
```

---

## 3. Extension Lifecycle

1. **Discovery & Registration**: Extensions are registered with `ExtensionRegistry::register(ext)`.
2. **Initialization (`on_init`)**: Validates prerequisites and initializes local resources.
3. **Event Observation (`on_event`)**: Subscribes to published domain events (e.g. `order.created`, `order.completed`, `payment.refunded`).
4. **Error Isolation**: Extension failures during event delivery log warnings and never terminate the host server or disrupt primary database transactions.
