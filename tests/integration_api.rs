use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use polygon_pol_indexer::api::AppState;
use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::models::{ProcessedTransfer, TransferDirection};
use serde_json::Value;
use std::sync::Arc;
use tower::util::ServiceExt;

/// Helper function to create a test database with sample data
async fn setup_test_database() -> Arc<Database> {
    let db = Database::new_in_memory().expect("Failed to create test database");
    
    // Add some test transactions
    let transfers = vec![
        ProcessedTransfer {
            block_number: 100,
            transaction_hash: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            log_index: 0,
            from_address: "0xsender1".to_string(),
            to_address: "0xf977814e90da44bfa03b6295a0616a897441acec".to_string(), // Binance
            amount: "1000.5".to_string(),
            timestamp: 1640995200, // 2022-01-01 00:00:00 UTC
            direction: TransferDirection::ToBinance,
        },
        ProcessedTransfer {
            block_number: 101,
            transaction_hash: "0xfedcba0987654321fedcba0987654321fedcba09".to_string(),
            log_index: 1,
            from_address: "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245".to_string(), // Binance
            to_address: "0xreceiver1".to_string(),
            amount: "500.25".to_string(),
            timestamp: 1640995260, // 2022-01-01 00:01:00 UTC
            direction: TransferDirection::FromBinance,
        },
        ProcessedTransfer {
            block_number: 102,
            transaction_hash: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            log_index: 0,
            from_address: "0xsender2".to_string(),
            to_address: "0x505e71695e9bc45943c58adec1650577bca68fd9".to_string(), // Binance
            amount: "2500.0".to_string(),
            timestamp: 1640995320, // 2022-01-01 00:02:00 UTC
            direction: TransferDirection::ToBinance,
        },
    ];

    for transfer in transfers {
        db.store_transfer_and_update_net_flow(&transfer)
            .expect("Failed to store test transfer");
    }

    // Update last processed block
    db.set_last_processed_block(102)
        .expect("Failed to set last processed block");

    Arc::new(db)
}

/// Helper function to create a test router
fn create_test_router(database: Arc<Database>) -> Router {
    use axum::routing::get;
    use polygon_pol_indexer::api::http::{get_net_flow, get_status, get_transactions};
    use tower::ServiceBuilder;
    use tower_http::cors::CorsLayer;

    let app_state = AppState { database };

    Router::new()
        .route("/net-flow", get(get_net_flow))
        .route("/status", get(get_status))
        .route("/transactions", get(get_transactions))
        .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
        .with_state(app_state)
}

#[tokio::test]
async fn test_get_net_flow_endpoint() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/net-flow")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response structure
    assert!(json.get("total_inflow").is_some());
    assert!(json.get("total_outflow").is_some());
    assert!(json.get("net_flow").is_some());
    assert!(json.get("last_processed_block").is_some());
    assert!(json.get("last_updated").is_some());

    // Verify values based on test data
    assert_eq!(json["total_inflow"], "3500.5"); // 1000.5 + 2500.0
    assert_eq!(json["total_outflow"], "500.25");
    assert_eq!(json["net_flow"], "3000.25"); // 3500.5 - 500.25
    assert_eq!(json["last_processed_block"], 102);
}

#[tokio::test]
async fn test_get_status_endpoint() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response structure
    assert!(json.get("status").is_some());
    assert!(json.get("last_processed_block").is_some());
    assert!(json.get("total_transactions").is_some());
    assert!(json.get("last_updated").is_some());
    assert!(json.get("database_status").is_some());

    // Verify values
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["last_processed_block"], 102);
    assert_eq!(json["total_transactions"], 3);
    assert_eq!(json["database_status"], "connected");
}

#[tokio::test]
async fn test_get_transactions_endpoint_default_params() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/transactions")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify response structure
    assert!(json.get("transactions").is_some());
    assert!(json.get("total_count").is_some());
    assert!(json.get("limit").is_some());
    assert!(json.get("offset").is_some());
    assert!(json.get("has_more").is_some());

    // Verify values
    assert_eq!(json["total_count"], 3);
    assert_eq!(json["limit"], 100); // default limit
    assert_eq!(json["offset"], 0); // default offset
    assert_eq!(json["has_more"], false); // no more data

    let transactions = json["transactions"].as_array().unwrap();
    assert_eq!(transactions.len(), 3);

    // Verify first transaction structure
    let first_tx = &transactions[0];
    assert!(first_tx.get("id").is_some());
    assert!(first_tx.get("block_number").is_some());
    assert!(first_tx.get("transaction_hash").is_some());
    assert!(first_tx.get("log_index").is_some());
    assert!(first_tx.get("from_address").is_some());
    assert!(first_tx.get("to_address").is_some());
    assert!(first_tx.get("amount").is_some());
    assert!(first_tx.get("timestamp").is_some());
    assert!(first_tx.get("direction").is_some());
    assert!(first_tx.get("created_at").is_some());
}

#[tokio::test]
async fn test_get_transactions_endpoint_with_limit() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/transactions?limit=2")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify pagination
    assert_eq!(json["total_count"], 3);
    assert_eq!(json["limit"], 2);
    assert_eq!(json["offset"], 0);
    assert_eq!(json["has_more"], true); // more data available

    let transactions = json["transactions"].as_array().unwrap();
    assert_eq!(transactions.len(), 2);
}

#[tokio::test]
async fn test_get_transactions_endpoint_with_offset() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/transactions?limit=2&offset=1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify pagination
    assert_eq!(json["total_count"], 3);
    assert_eq!(json["limit"], 2);
    assert_eq!(json["offset"], 1);
    assert_eq!(json["has_more"], false); // no more data after this page

    let transactions = json["transactions"].as_array().unwrap();
    assert_eq!(transactions.len(), 2); // Should return 2 transactions (offset 1, limit 2)
}

#[tokio::test]
async fn test_get_transactions_endpoint_invalid_limit_zero() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/transactions?limit=0")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"], "invalid_parameter");
    assert!(json["message"].as_str().unwrap().contains("greater than 0"));
}

#[tokio::test]
async fn test_get_transactions_endpoint_invalid_limit_too_high() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/transactions?limit=1001")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"], "invalid_parameter");
    assert!(json["message"].as_str().unwrap().contains("cannot exceed 1000"));
}

#[tokio::test]
async fn test_get_transactions_endpoint_high_offset() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/transactions?limit=10&offset=100")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Should return empty transactions array but still be successful
    assert_eq!(json["total_count"], 3);
    assert_eq!(json["limit"], 10);
    assert_eq!(json["offset"], 100);
    assert_eq!(json["has_more"], false);

    let transactions = json["transactions"].as_array().unwrap();
    assert_eq!(transactions.len(), 0);
}

#[tokio::test]
async fn test_endpoints_with_empty_database() {
    let db = Database::new_in_memory().expect("Failed to create test database");
    let database = Arc::new(db);
    let app = create_test_router(database);

    // Test net-flow endpoint with empty database
    let request = Request::builder()
        .uri("/net-flow")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Should return zero values for empty database
    assert_eq!(json["total_inflow"], "0");
    assert_eq!(json["total_outflow"], "0");
    assert_eq!(json["net_flow"], "0");
    assert_eq!(json["last_processed_block"], 0);
}

#[tokio::test]
async fn test_status_endpoint_with_empty_database() {
    let db = Database::new_in_memory().expect("Failed to create test database");
    let database = Arc::new(db);
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
    assert_eq!(json["last_processed_block"], 0);
    assert_eq!(json["total_transactions"], 0);
    assert_eq!(json["database_status"], "connected");
}

#[tokio::test]
async fn test_transactions_endpoint_with_empty_database() {
    let db = Database::new_in_memory().expect("Failed to create test database");
    let database = Arc::new(db);
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/transactions")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["total_count"], 0);
    assert_eq!(json["limit"], 100);
    assert_eq!(json["offset"], 0);
    assert_eq!(json["has_more"], false);

    let transactions = json["transactions"].as_array().unwrap();
    assert_eq!(transactions.len(), 0);
}

#[tokio::test]
async fn test_cors_headers() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/net-flow")
        .header("Origin", "http://localhost:3000")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    // CORS headers should be present due to CorsLayer::permissive()
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_invalid_endpoint() {
    let database = setup_test_database().await;
    let app = create_test_router(database);

    let request = Request::builder()
        .uri("/invalid-endpoint")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}