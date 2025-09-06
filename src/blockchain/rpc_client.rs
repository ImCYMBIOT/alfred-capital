use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("RPC error: {0}")]
    Rpc(String),
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Vec<Value>,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

pub struct RpcClient {
    client: Client,
    endpoint: String,
}

impl RpcClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
        }
    }

    async fn make_request(&self, method: &str, params: Vec<Value>) -> Result<Value, RpcError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: 1,
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await?;

        let rpc_response: JsonRpcResponse = response.json().await?;

        if let Some(error) = rpc_response.error {
            return Err(RpcError::Rpc(format!(
                "Code: {}, Message: {}",
                error.code, error.message
            )));
        }

        rpc_response
            .result
            .ok_or_else(|| RpcError::Rpc("No result in response".to_string()))
    }

    pub async fn get_latest_block_number(&self) -> Result<u64, RpcError> {
        let result = self.make_request("eth_blockNumber", vec![]).await?;
        
        let hex_string = result
            .as_str()
            .ok_or_else(|| RpcError::Rpc("Block number is not a string".to_string()))?;

        // Remove "0x" prefix and parse as hex
        let hex_without_prefix = hex_string.strip_prefix("0x").unwrap_or(hex_string);
        u64::from_str_radix(hex_without_prefix, 16)
            .map_err(|e| RpcError::Rpc(format!("Failed to parse block number: {}", e)))
    }
}
#
[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_rpc_client_creation() {
        let endpoint = "https://polygon-rpc.com/".to_string();
        let client = RpcClient::new(endpoint.clone());
        assert_eq!(client.endpoint, endpoint);
    }

    #[tokio::test]
    async fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_blockNumber".to_string(),
            params: vec![],
            id: 1,
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let expected = r#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}"#;
        assert_eq!(serialized, expected);
    }

    #[tokio::test]
    async fn test_json_rpc_response_deserialization_success() {
        let response_json = r#"{"jsonrpc":"2.0","result":"0x1234","id":1}"#;
        let response: JsonRpcResponse = serde_json::from_str(response_json).unwrap();
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.result.unwrap(), json!("0x1234"));
    }

    #[tokio::test]
    async fn test_json_rpc_response_deserialization_error() {
        let response_json = r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Method not found"},"id":1}"#;
        let response: JsonRpcResponse = serde_json::from_str(response_json).unwrap();
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
    }

    #[test]
    fn test_hex_block_number_parsing() {
        // Test parsing hex block numbers (without actual RPC call)
        let test_cases = vec![
            ("0x1234", 0x1234u64),
            ("0xabc", 0xabcu64),
            ("0x0", 0u64),
            ("1234", 0x1234u64), // Without 0x prefix
        ];

        for (hex_str, expected) in test_cases {
            let hex_without_prefix = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            let result = u64::from_str_radix(hex_without_prefix, 16).unwrap();
            assert_eq!(result, expected, "Failed to parse {}", hex_str);
        }
    }

    // Mock server test would require additional dependencies like wiremock
    // For now, we'll test the parsing logic and structure
    #[test]
    fn test_rpc_error_display() {
        let rpc_error = RpcError::Rpc("Custom error".to_string());
        assert_eq!(format!("{}", rpc_error), "RPC error: Custom error");
    }
}