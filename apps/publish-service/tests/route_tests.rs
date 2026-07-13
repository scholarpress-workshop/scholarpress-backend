use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use tower::ServiceExt;

const MOCK_SPEC_YAML: &str = r#"
institution: Test University
source_revision: "2026-01"
document_structure:
  front_matter:
    - { id: title_page, required: true }
  body:
    - { id: chapters, required: true }
  end_matter:
    - { id: references, required: true }
checks:
  - id: global_margins
    category: layout
    checker: margins
    target: { scope: all_pages }
    params:
      top: 1in
      bottom: 1in
      left: 1.25in
      right: 1.25in
  - id: committee_order
    category: content
    checker: committee_order
    target: { page: acceptance }
    automatable: false
    review_hint: "Check committee order on acceptance page"
constants:
  degree: "Doctor of Philosophy"
"#;

fn test_app() -> (axum::Router, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let inst_dir = tmp.path().join("institutions").join("test");
    std::fs::create_dir_all(&inst_dir).unwrap();
    std::fs::write(inst_dir.join("spec.yaml"), MOCK_SPEC_YAML).unwrap();
    std::fs::create_dir_all(inst_dir.join("template")).unwrap();
    std::fs::write(
        inst_dir.join("template").join("template.typ"),
        "#set page(width: 100pt, height: 100pt); \"hello\"",
    )
    .unwrap();

    let config = publish_service::config::AppConfig {
        port: 0,
        catalog_path: tmp.path().to_path_buf(),
    };
    let registry = publish_service::institutions::Registry::load(&config.catalog_path).unwrap();
    let router = publish_service::routes::router(registry);
    (router, tmp)
}

#[tokio::test]
async fn test_health_returns_ok() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn test_institutions_lists_ids() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/institutions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "test");
    assert_eq!(arr[0]["name"], "Test University");
}

#[tokio::test]
async fn test_spec_returns_yaml() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/institutions/test/spec")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["yaml"].as_str().unwrap().contains("Test University"));
    assert_eq!(json["summary"]["automated_checks"], 1);
    assert_eq!(json["summary"]["human_checks"], 1);
}

#[tokio::test]
async fn test_spec_not_found() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/institutions/nonexistent/spec")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Institution not found"));
}

#[tokio::test]
async fn test_template_returns_files() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/institutions/test/template")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["entry"], "template.typ");
    assert!(!json["files"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_template_not_found() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/institutions/nonexistent/template")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_extract_no_file_returns_error() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/extract")
                .header("content-type", "multipart/form-data; boundary=xxx")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_server_error() || response.status().is_client_error());
    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(
        text.contains("No file") || text.contains("error"),
        "expected error message, got: {text}"
    );
}

#[tokio::test]
async fn test_validate_invalid_base64() {
    let (app, _tmp) = test_app();
    let body = serde_json::json!({
        "pdf_base64": "!!!not-valid-base64!!!",
        "institution": "test"
    })
    .to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/validate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let resp_body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&resp_body).unwrap();
    assert!(json["error"].as_str().unwrap().contains("Invalid base64"));
}

#[tokio::test]
async fn test_validate_missing_institution() {
    let (app, _tmp) = test_app();
    let body = serde_json::json!({
        "pdf_base64": "dGVzdA==",
        "institution": "nonexistent"
    })
    .to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/validate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_compile_institution_not_found() {
    let (app, _tmp) = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/compile?institution=nonexistent")
                .header("content-type", "application/json")
                .body(Body::from(
                    r##"{"typst_code": "#set page(width: 100pt, height: 100pt); \"hello\""}"##,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), 10_000)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Institution not found"));
}
