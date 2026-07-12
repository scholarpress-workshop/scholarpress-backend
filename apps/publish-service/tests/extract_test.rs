use std::net::SocketAddr;

#[tokio::test]
async fn test_extract_pdf() {
    let addr = start_test_server().await;
    let pdf_bytes = include_bytes!("../../fixtures/test-dissertation.pdf");
    let client = reqwest::Client::new();

    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(pdf_bytes.to_vec())
            .file_name("dissertation.pdf")
            .mime_str("application/pdf")
            .unwrap(),
    );

    let resp = client
        .post(format!("http://{}/extract?institution=iu", addr))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.get("content").is_some());
    assert!(body.get("structure").is_some());
    assert!(body.get("metadata").is_some());
    assert!(body["structure"].get("headings").is_some());
    assert!(body["metadata"].get("page_count_estimated").is_some());
}

async fn start_test_server() -> SocketAddr {
    let institutions = doc_service::institutions::Registry::load(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("institutions"),
    )
    .await
    .unwrap();

    let app = doc_service::routes::router()
        .with_state(institutions)
        .layer(tower_http::cors::CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}
