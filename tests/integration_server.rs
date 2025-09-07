use polygon_pol_indexer::api::ApiServer;
use polygon_pol_indexer::database::Database;
use std::sync::Arc;

#[tokio::test]
async fn test_api_server_creation() {
    // Test that we can create an API server instance
    let database = Database::new_in_memory().expect("Failed to create test database");
    let database = Arc::new(database);
    
    let server = ApiServer::new(database, 8080);
    
    // Just verify the server was created successfully
    // We don't actually start it since that would block the test
    assert_eq!(server.port, 8080);
}

#[tokio::test]
async fn test_api_server_with_different_port() {
    let database = Database::new_in_memory().expect("Failed to create test database");
    let database = Arc::new(database);
    
    let server = ApiServer::new(database, 3000);
    assert_eq!(server.port, 3000);
}