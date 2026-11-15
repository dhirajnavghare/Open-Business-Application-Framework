use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use obaf_api::{create_router, docs::ApiDoc, AppState};
use obaf_database::{establish_connection, run_migrations};
use std::sync::Arc;
use tower::ServiceExt;
use utoipa::OpenApi;

#[test]
fn test_openapi_spec_validity_and_ref_integrity() {
    let openapi = ApiDoc::openapi();
    let json_val = serde_json::to_value(&openapi).expect("serialize OpenAPI to JSON");

    assert!(
        json_val.get("openapi").is_some(),
        "Must have openapi version"
    );
    assert!(json_val.get("info").is_some(), "Must have info block");
    assert!(json_val.get("paths").is_some(), "Must have paths block");
    assert!(
        json_val.get("components").is_some(),
        "Must have components block"
    );

    let paths = json_val["paths"].as_object().expect("paths is object");
    assert!(!paths.is_empty(), "Paths must not be empty");

    let components = json_val["components"]
        .as_object()
        .expect("components is object");
    let schemas = components["schemas"]
        .as_object()
        .expect("schemas is object");

    println!("Total documented OpenAPI paths: {}", paths.len());
    println!("Total documented OpenAPI schemas: {}", schemas.len());

    // Recursively collect all "$ref" strings in the entire JSON tree
    fn collect_refs(val: &serde_json::Value, refs: &mut Vec<String>) {
        match val {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    if k == "$ref" {
                        if let Some(ref_str) = v.as_str() {
                            refs.push(ref_str.to_string());
                        }
                    } else {
                        collect_refs(v, refs);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    collect_refs(item, refs);
                }
            }
            _ => {}
        }
    }

    let mut all_refs = Vec::new();
    collect_refs(&json_val, &mut all_refs);
    println!("Total $ref links in OpenAPI spec: {}", all_refs.len());

    let mut missing_refs = Vec::new();
    for r in &all_refs {
        if let Some(schema_name) = r.strip_prefix("#/components/schemas/") {
            if !schemas.contains_key(schema_name) {
                missing_refs.push(r.clone());
            }
        }
    }

    assert!(
        missing_refs.is_empty(),
        "OpenAPI spec contains unresolved $ref schemas: {:?}",
        missing_refs
    );

    let tags = json_val["tags"].as_array().expect("tags is array");
    assert!(
        tags.len() >= 15,
        "Expected at least 15 API tags, got {}",
        tags.len()
    );
}

#[tokio::test]
async fn test_swagger_ui_and_openapi_endpoints_served() {
    let db = establish_connection("sqlite::memory:")
        .await
        .expect("in-memory db");
    run_migrations(&db).await.expect("migrations");

    let state = Arc::new(AppState::new(
        db.clone(),
        "test_jwt_secret_docs_32_bytes_min",
    ));
    let app = create_router(state);

    // 1. Test OpenAPI JSON endpoint (/api/v1/openapi.json)
    let openapi_req = Request::builder()
        .method("GET")
        .uri("/api/v1/openapi.json")
        .body(Body::empty())
        .unwrap();

    let openapi_res = app.clone().oneshot(openapi_req).await.unwrap();
    assert_eq!(openapi_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(openapi_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        json_val["info"]["title"],
        "Open Business Application Framework (OBAF) API"
    );
    assert_eq!(json_val["info"]["version"], "0.1.0");

    // 2. Test Dynamic Introspection Info endpoint (/api/v1/meta/info)
    let info_req = Request::builder()
        .method("GET")
        .uri("/api/v1/meta/info")
        .body(Body::empty())
        .unwrap();

    let info_res = app.clone().oneshot(info_req).await.unwrap();
    assert_eq!(info_res.status(), StatusCode::OK);
    let info_bytes = axum::body::to_bytes(info_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let info_json: serde_json::Value = serde_json::from_slice(&info_bytes).unwrap();
    assert_eq!(
        info_json["name"],
        "Open Business Application Framework (OBAF)"
    );
    let presets = info_json["available_presets"].as_array().unwrap();
    assert_eq!(
        presets.len(),
        22,
        "All 22 domain presets must be declared in meta info"
    );

    // 3. Test Dynamic Introspection Schema Discovery (/api/v1/meta/schema)
    let schema_req = Request::builder()
        .method("GET")
        .uri("/api/v1/meta/schema")
        .body(Body::empty())
        .unwrap();

    let schema_res = app.clone().oneshot(schema_req).await.unwrap();
    assert_eq!(schema_res.status(), StatusCode::OK);
    let schema_bytes = axum::body::to_bytes(schema_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let schema_json: serde_json::Value = serde_json::from_slice(&schema_bytes).unwrap();
    let entities = schema_json["entities"].as_array().unwrap();
    assert!(
        entities.len() >= 10,
        "Expected at least 10 discovered entities in schema discovery"
    );
}
