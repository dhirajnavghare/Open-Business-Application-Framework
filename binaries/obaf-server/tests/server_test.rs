use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::time::Duration;

fn get_workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

struct ProcessGuard {
    child: Child,
    db_path: String,
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.db_path);
        let _ = fs::remove_file(format!("{}-shm", self.db_path));
        let _ = fs::remove_file(format!("{}-wal", self.db_path));
    }
}

#[tokio::test]
async fn test_server_binary_startup_and_endpoints() {
    let test_port: u16 = 8992;
    let db_path = "/tmp/test_server_binary_integration.db".to_string();
    let db_url = format!("sqlite://{}?mode=rwc", db_path);

    let _ = fs::remove_file(&db_path);

    let bin = env!("CARGO_BIN_EXE_obaf-server");
    let child = Command::new(bin)
        .current_dir(get_workspace_root())
        .env("PORT", test_port.to_string())
        .env("HOST", "127.0.0.1")
        .env("DATABASE_URL", &db_url)
        .spawn()
        .expect("Failed to start obaf-server binary");

    let _guard = ProcessGuard { child, db_path };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let health_url = format!("http://127.0.0.1:{}/health", test_port);
    let openapi_url = format!("http://127.0.0.1:{}/api/v1/openapi.json", test_port);
    let swagger_url = format!("http://127.0.0.1:{}/swagger-ui/", test_port);

    // Poll until healthy
    let mut ready = false;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(150)).await;
        if let Ok(resp) = client.get(&health_url).send().await {
            if resp.status().is_success() {
                ready = true;
                break;
            }
        }
    }

    assert!(
        ready,
        "obaf-server failed to become ready on port {}",
        test_port
    );

    // 1. Check /health
    let health_res = client.get(&health_url).send().await.unwrap();
    assert_eq!(health_res.status(), reqwest::StatusCode::OK);
    let health_json: serde_json::Value = health_res.json().await.unwrap();
    assert_eq!(health_json["status"], "ok");
    assert_eq!(health_json["database"], "connected");

    // 2. Check /api/v1/openapi.json
    let openapi_res = client.get(&openapi_url).send().await.unwrap();
    assert_eq!(openapi_res.status(), reqwest::StatusCode::OK);
    let openapi_json: serde_json::Value = openapi_res.json().await.unwrap();
    assert!(openapi_json.get("openapi").is_some());
    assert!(openapi_json.get("paths").is_some());

    // 3. Check /swagger-ui/
    let swagger_res = client.get(&swagger_url).send().await.unwrap();
    assert_eq!(swagger_res.status(), reqwest::StatusCode::OK);

    // 4. Check /llms.txt AI Discovery endpoint
    let llms_url = format!("http://127.0.0.1:{}/llms.txt", test_port);
    let llms_res = client.get(&llms_url).send().await.unwrap();
    assert_eq!(llms_res.status(), reqwest::StatusCode::OK);
    let llms_text = llms_res.text().await.unwrap();
    assert!(llms_text.contains("Open Business Application Framework (OBAF)"));
    assert!(llms_text.contains("Core Capabilities"));

    // 5. Check /live liveness probe
    let live_url = format!("http://127.0.0.1:{}/live", test_port);
    let live_res = client.get(&live_url).send().await.unwrap();
    assert_eq!(live_res.status(), reqwest::StatusCode::OK);
    let live_json: serde_json::Value = live_res.json().await.unwrap();
    assert_eq!(live_json["status"], "alive");

    // 6. Check /ready readiness probe
    let ready_url = format!("http://127.0.0.1:{}/ready", test_port);
    let ready_res = client.get(&ready_url).send().await.unwrap();
    assert_eq!(ready_res.status(), reqwest::StatusCode::OK);
    let ready_json: serde_json::Value = ready_res.json().await.unwrap();
    assert_eq!(ready_json["status"], "ready");
    assert_eq!(ready_json["database"], "connected");
}
