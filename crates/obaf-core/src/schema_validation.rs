use crate::error::{ObafError, Result};
use jsonschema::validator_for;
use serde_json::Value;

/// Validates an incoming instance JSON object against a JSON Schema.
pub fn validate_metadata(schema: &Value, instance: &Value) -> Result<()> {
    let validator = validator_for(schema)
        .map_err(|e| ObafError::validation(format!("Invalid JSON Schema definition: {}", e)))?;

    let mut errors = Vec::new();
    for error in validator.iter_errors(instance) {
        errors.push(error.to_string());
    }

    if !errors.is_empty() {
        return Err(ObafError::validation(format!(
            "Custom field metadata validation failed: {}",
            errors.join("; ")
        )));
    }

    Ok(())
}
