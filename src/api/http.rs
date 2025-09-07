use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};

use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

use crate::database::{Database, DbError};

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] DbError),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Server error: {0}")]
    Server(String),
}

impl From<ApiError> for StatusCode {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::InvalidParameter(_) => StatusCode::BAD_REQUEST,
            ApiError::Server(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Response structure for net-flow endpoint
#[derive(Debug, Serialize)]
pub struct NetFlowResponse {
    pub total_inflow: String,
    pub total_outflow: String,
    pub net_flow: String,
    pub last_processed_block: u64,
    pub last_updated: u64,
}

/// Response structure for status endpoint
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub last_processed_block: u64,
    pub total_transactions: u64,
    pub last_updated: u64,
    pub database_status: String,
}

/// Response structure for individual transaction
#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub id: i64,
    pub block_number: u64,
    pub transaction_hash: String,
    pub log_index: u32,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub timestamp: u64,
    pub direction: String,
    pub created_at: u64,
}

/// Response structure for transactions endpoint
#[derive(Debug, Serialize)]
pub struct TransactionsResponse {
    pub transactions: Vec<TransactionResponse>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
    pub has_more: bool,
}

/// Query parameters for transactions endpoint
#[derive(Debug, Deserialize)]
pub struct TransactionsQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    100
}

/// Error response structure
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub database: Arc<Database>,
}

/// HTTP API server
pub struct ApiServer {
    database: Arc<Database>,
    pub port: u16,
}

impl ApiServer {
    /// Create a new API server instance
    pub fn new(database: Arc<Database>, port: u16) -> Self {
        Self { database, port }
    }

    /// Start the HTTP server
    pub async fn start(&self) -> Result<(), ApiError> {
        let app_state = AppState {
            database: self.database.clone(),
        };

        let app = Router::new()
            .route("/net-flow", get(get_net_flow))
            .route("/status", get(get_status))
            .route("/transactions", get(get_transactions))
            .layer(
                ServiceBuilder::new()
                    .layer(CorsLayer::permissive())
            )
            .with_state(app_state);

        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| ApiError::Server(format!("Failed to bind to {}: {}", addr, e)))?;

        log::info!("HTTP API server starting on {}", addr);

        axum::serve(listener, app)
            .await
            .map_err(|e| ApiError::Server(format!("Server error: {}", e)))?;

        Ok(())
    }
}

/// GET /net-flow - Get current cumulative net-flow data
pub async fn get_net_flow(
    State(state): State<AppState>,
) -> Result<Json<NetFlowResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.database.get_net_flow_data() {
        Ok(net_flow_data) => {
            let response = NetFlowResponse {
                total_inflow: net_flow_data.total_inflow,
                total_outflow: net_flow_data.total_outflow,
                net_flow: net_flow_data.net_flow,
                last_processed_block: net_flow_data.last_processed_block,
                last_updated: net_flow_data.last_updated,
            };
            Ok(Json(response))
        }
        Err(e) => {
            log::error!("Failed to get net-flow data: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "database_error".to_string(),
                    message: format!("Failed to retrieve net-flow data: {}", e),
                }),
            ))
        }
    }
}

/// GET /status - Get system status and health information
pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    match (
        state.database.get_net_flow_data(),
        state.database.get_transaction_count(),
    ) {
        (Ok(net_flow_data), Ok(transaction_count)) => {
            let response = StatusResponse {
                status: "healthy".to_string(),
                last_processed_block: net_flow_data.last_processed_block,
                total_transactions: transaction_count,
                last_updated: net_flow_data.last_updated,
                database_status: "connected".to_string(),
            };
            Ok(Json(response))
        }
        (Err(e), _) | (_, Err(e)) => {
            log::error!("Failed to get status data: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "database_error".to_string(),
                    message: format!("Failed to retrieve status data: {}", e),
                }),
            ))
        }
    }
}

/// GET /transactions - Get recent transactions with pagination
pub async fn get_transactions(
    Query(params): Query<TransactionsQuery>,
    State(state): State<AppState>,
) -> Result<Json<TransactionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate parameters
    if params.limit == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_parameter".to_string(),
                message: "Limit must be greater than 0".to_string(),
            }),
        ));
    }

    if params.limit > 1000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_parameter".to_string(),
                message: "Limit cannot exceed 1000".to_string(),
            }),
        ));
    }

    match (
        state.database.get_recent_transactions(params.limit, params.offset),
        state.database.get_transaction_count(),
    ) {
        (Ok(transactions), Ok(total_count)) => {
            let transaction_responses: Vec<TransactionResponse> = transactions
                .into_iter()
                .map(|tx| TransactionResponse {
                    id: tx.id,
                    block_number: tx.block_number,
                    transaction_hash: tx.transaction_hash,
                    log_index: tx.log_index,
                    from_address: tx.from_address,
                    to_address: tx.to_address,
                    amount: tx.amount,
                    timestamp: tx.timestamp,
                    direction: tx.direction,
                    created_at: tx.created_at,
                })
                .collect();

            let has_more = (params.offset + params.limit) < total_count as u32;

            let response = TransactionsResponse {
                transactions: transaction_responses,
                total_count,
                limit: params.limit,
                offset: params.offset,
                has_more,
            };

            Ok(Json(response))
        }
        (Err(e), _) | (_, Err(e)) => {
            log::error!("Failed to get transactions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "database_error".to_string(),
                    message: format!("Failed to retrieve transactions: {}", e),
                }),
            ))
        }
    }
}