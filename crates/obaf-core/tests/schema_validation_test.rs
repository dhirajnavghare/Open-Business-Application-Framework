use obaf_core::validate_metadata;
use serde_json::json;

#[test]
fn test_valid_custom_field_metadata() {
    let schema = json!({
        "type": "object",
        "properties": {
            "doctor_name": { "type": "string" },
            "batch_number": { "type": "string" },
            "dosage_mg": { "type": "number" }
        },
        "required": ["doctor_name", "batch_number"]
    });

    let valid_data = json!({
        "doctor_name": "Dr. Smith",
        "batch_number": "BATCH-901",
        "dosage_mg": 500
    });

    assert!(validate_metadata(&schema, &valid_data).is_ok());
}

#[test]
fn test_invalid_custom_field_metadata() {
    let schema = json!({
        "type": "object",
        "properties": {
            "doctor_name": { "type": "string" },
            "batch_number": { "type": "string" }
        },
        "required": ["doctor_name", "batch_number"]
    });

    // Missing required batch_number
    let invalid_data = json!({
        "doctor_name": "Dr. Smith"
    });

    let result = validate_metadata(&schema, &invalid_data);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("validation failed"));
}
