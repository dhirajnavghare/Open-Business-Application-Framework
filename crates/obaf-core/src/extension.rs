use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use utoipa::ToSchema;

use crate::error::{ObafError, Result};
use crate::events::DomainEvent;

/// Metadata describing an open-source or custom domain extension.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExtensionManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub required_modules: Vec<String>,
}

/// Context provided to extensions during initialization.
#[derive(Debug, Clone, Default)]
pub struct ExtensionContext {
    pub environment: String,
    pub config: serde_json::Value,
}

/// Contract that external community modules and domain plugins implement.
pub trait ObafExtension: Send + Sync {
    /// Returns the extension's declarative manifest.
    fn manifest(&self) -> ExtensionManifest;

    /// Lifecycle hook called when the extension is registered and loaded by the engine.
    fn on_init(&self, ctx: &ExtensionContext) -> Result<()>;

    /// Event hook called whenever a domain event is published across the platform.
    fn on_event(&self, event: &DomainEvent<serde_json::Value>) -> Result<()>;
}

/// Thread-safe registry for dynamically managing active platform extensions.
#[derive(Default, Clone)]
pub struct ExtensionRegistry {
    extensions: Arc<RwLock<Vec<Arc<dyn ObafExtension>>>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a new extension into the runtime.
    pub fn register<E: ObafExtension + 'static>(&self, extension: E) -> Result<()> {
        let ctx = ExtensionContext::default();
        extension.on_init(&ctx)?;
        let mut lock = self.extensions.write().map_err(|_| ObafError::Internal {
            message: "ExtensionRegistry lock poisoned".into(),
        })?;
        lock.push(Arc::new(extension));
        Ok(())
    }

    /// List all registered extension manifests.
    pub fn list_manifests(&self) -> Vec<ExtensionManifest> {
        let lock = self.extensions.read().unwrap_or_else(|e| e.into_inner());
        lock.iter().map(|ext| ext.manifest()).collect()
    }

    /// Dispatch a published domain event to all registered extensions.
    pub fn dispatch_event(&self, event: &DomainEvent<serde_json::Value>) -> Result<()> {
        let exts = {
            let lock = self.extensions.read().unwrap_or_else(|e| e.into_inner());
            lock.clone()
        };
        for ext in exts {
            if let Err(e) = ext.on_event(event) {
                tracing::warn!(
                    extension = %ext.manifest().name,
                    error = %e,
                    "Extension returned an error while handling domain event"
                );
            }
        }
        Ok(())
    }
}
