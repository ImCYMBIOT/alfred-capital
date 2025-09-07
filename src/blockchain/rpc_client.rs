use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use crate::models::RawLog;
use crate::error::{IndexerError, RpcError as NewRpcError};
use crate::logging::{LogContext, PerformanceMonitor, MetricsLogger};
use crate::retry::RetryUtils;

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

#[derive(Debug, Deserialize)]
pub struct Block {
    pub number: String,
    pub hash: String,
    pub timestamp: String,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    #[serde(rename = "blockNumber")]
    pub block_number: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogFilter {
    #[serde(rename = "fromBlock")]
    pub from_block: String,
    #[serde(rename = "toBlock")]
    pub to_block: String,
    pub address: Option<String>,
    pub topics: Option<Vec<Option<String>>>,
}

#[derive(Debug, Deserialize)]
pub struct EthLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    #[serde(rename = "blockNumber")]
    pub block_number: String,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "logIndex")]
    pub log_index: String,
}

#[derive(Clone)]
pub struct RpcClient {
    client: Client,
    endpoint: String,
}

impl RpcClient {
    pub fn new(endpoint: String) -> Self {
        let context = LogContext::new("rpc_client", "initialization")
            .with_metadata("endpoint", serde_json::json!(endpoint));
        context.info("Initializing RPC client");
        
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            endpoint,
        }
    }

    /// Enhanced RPC client with timeout and connection pooling
    pub fn new_with_config(endpoint: String, timeout_seconds: u64) -> Self {
        let context = LogContext::new("rpc_client", "initialization")
            .with_metadata("endpoint", serde_json::json!(endpoint))
            .with_metadata("timeout_seconds", serde_json::json!(timeout_seconds));
        context.info("Initializing RPC client with custom configuration");
        
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_seconds))
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
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

    /// Enhanced make_request with better error handling and logging
    async fn make_request_enhanced(&self, method: &str, params: Vec<Value>) -> Result<Value, IndexerError> {
        let context = LogContext::new("rpc_client", "make_request")
            .with_metadata("method", serde_json::json!(method))
            .with_metadata("endpoint", serde_json::json!(self.endpoint));

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: 1,
        };

        context.trace(&format!("Sending RPC request: {}", method));

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                // Classify HTTP errors
                if e.is_timeout() {
                    IndexerError::Rpc(NewRpcError::Timeout { seconds: 30 })
                } else if e.is_connect() {
                    IndexerError::Rpc(NewRpcError::Connection(e.to_string()))
                } else if e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
                    IndexerError::Rpc(NewRpcError::RateLimit { seconds: 60 })
                } else {
                    IndexerError::Rpc(NewRpcError::Http(e))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_msg = format!("HTTP error: {} {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown"));
            return Err(IndexerError::Rpc(NewRpcError::Connection(error_msg)));
        }

        let rpc_response: JsonRpcResponse = response.json().await
            .map_err(|e| IndexerError::Rpc(NewRpcError::Http(e)))?;

        if let Some(error) = rpc_response.error {
            let rpc_error = match error.code {
                -32700 => NewRpcError::InvalidResponse("Parse error".to_string()),
                -32600 => NewRpcError::InvalidResponse("Invalid request".to_string()),
                -32601 => NewRpcError::Method { code: error.code, message: error.message },
                -32602 => NewRpcError::InvalidResponse("Invalid params".to_string()),
                -32603 => NewRpcError::Method { code: error.code, message: error.message },
                _ => NewRpcError::Method { code: error.code, message: error.message },
            };
            return Err(IndexerError::Rpc(rpc_error));
        }

        rpc_response
            .result
            .ok_or_else(|| IndexerError::Rpc(NewRpcError::InvalidResponse("No result in response".to_string())))
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

    /// Enhanced version with retry logic and better error handling
    pub async fn get_latest_block_number_with_retry(&self) -> Result<u64, IndexerError> {
        RetryUtils::retry_rpc("get_latest_block_number", || async {
            let monitor = PerformanceMonitor::new("rpc_get_latest_block_number");
            
            let result = self.make_request_enhanced("eth_blockNumber", vec![]).await;
            let duration = monitor.finish_with_result(&result);
            
            MetricsLogger::log_rpc_call("eth_blockNumber", duration, result.is_ok());
            
            match result {
                Ok(value) => {
                    let hex_string = value
                        .as_str()
                        .ok_or_else(|| IndexerError::Rpc(NewRpcError::InvalidResponse(
                            "Block number is not a string".to_string()
                        )))?;

                    let hex_without_prefix = hex_string.strip_prefix("0x").unwrap_or(hex_string);
                    let block_number = u64::from_str_radix(hex_without_prefix, 16)
                        .map_err(|e| IndexerError::Processing(
                            crate::error::ProcessingError::BlockParsing(
                                format!("Failed to parse block number: {}", e)
                            )
                        ))?;

                    let context = LogContext::new("rpc_client", "get_latest_block_number")
                        .with_block_number(block_number);
                    context.debug(&format!("Retrieved latest block number: {}", block_number));

                    Ok(block_number)
                }
                Err(e) => Err(e),
            }
        }).await
    }

    pub async fn get_block(&self, block_number: u64) -> Result<Block, RpcError> {
        let block_hex = format!("0x{:x}", block_number);
        let params = vec![
            serde_json::Value::String(block_hex),
            serde_json::Value::Bool(true), // Include full transaction objects
        ];
        
        let result = self.make_request("eth_getBlockByNumber", params).await?;
        
        if result.is_null() {
            return Err(RpcError::Rpc(format!("Block {} not found", block_number)));
        }
        
        serde_json::from_value(result)
            .map_err(|e| RpcError::Json(e))
    }

    /// Enhanced version with retry logic and better error handling
    pub async fn get_block_with_retry(&self, block_number: u64) -> Result<Block, IndexerError> {
        RetryUtils::retry_rpc("get_block", || async {
            let monitor = PerformanceMonitor::new("rpc_get_block")
                .with_metadata("block_number", serde_json::json!(block_number));
            
            let block_hex = format!("0x{:x}", block_number);
            let params = vec![
                serde_json::Value::String(block_hex),
                serde_json::Value::Bool(true), // Include full transaction objects
            ];
            
            let result = self.make_request_enhanced("eth_getBlockByNumber", params).await;
            let duration = monitor.finish_with_result(&result);
            
            MetricsLogger::log_rpc_call("eth_getBlockByNumber", duration, result.is_ok());
            
            match result {
                Ok(value) => {
                    if value.is_null() {
                        return Err(IndexerError::Rpc(NewRpcError::BlockNotFound { block_number }));
                    }
                    
                    let block: Block = serde_json::from_value(value)
                        .map_err(|e| IndexerError::Processing(
                            crate::error::ProcessingError::BlockParsing(
                                format!("Failed to parse block {}: {}", block_number, e)
                            )
                        ))?;

                    let context = LogContext::new("rpc_client", "get_block")
                        .with_block_number(block_number)
                        .with_metadata("transaction_count", serde_json::json!(block.transactions.len()));
                    context.debug(&format!("Retrieved block {} with {} transactions", 
                        block_number, block.transactions.len()));

                    Ok(block)
                }
                Err(e) => Err(e),
            }
        }).await
    }

    pub async fn get_logs(&self, filter: LogFilter) -> Result<Vec<RawLog>, RpcError> {
        let params = vec![serde_json::to_value(filter)?];
        let result = self.make_request("eth_getLogs", params).await?;
        
        let eth_logs: Vec<EthLog> = serde_json::from_value(result)?;
        
        // Convert EthLog to RawLog
        let raw_logs = eth_logs.into_iter().map(|eth_log| {
            let block_number = parse_hex_to_u64(&eth_log.block_number).unwrap_or(0);
            let log_index = parse_hex_to_u32(&eth_log.log_index).unwrap_or(0);
            
            RawLog {
                address: eth_log.address,
                topics: eth_log.topics,
                data: eth_log.data,
                block_number,
                transaction_hash: eth_log.transaction_hash,
                log_index,
            }
        }).collect();
        
        Ok(raw_logs)
    }

    /// Enhanced version with retry logic and better error handling
    pub async fn get_logs_with_retry(&self, filter: LogFilter) -> Result<Vec<RawLog>, IndexerError> {
        RetryUtils::retry_rpc("get_logs", || async {
            let monitor = PerformanceMonitor::new("rpc_get_logs")
                .with_metadata("from_block", serde_json::json!(filter.from_block))
                .with_metadata("to_block", serde_json::json!(filter.to_block));
            
            let params = vec![serde_json::to_value(&filter)
                .map_err(|e| IndexerError::Rpc(NewRpcError::Json(e)))?];
            
            let result = self.make_request_enhanced("eth_getLogs", params).await;
            let duration = monitor.finish_with_result(&result);
            
            MetricsLogger::log_rpc_call("eth_getLogs", duration, result.is_ok());
            
            match result {
                Ok(value) => {
                    let eth_logs: Vec<EthLog> = serde_json::from_value(value)
                        .map_err(|e| IndexerError::Processing(
                            crate::error::ProcessingError::LogParsing(
                                format!("Failed to parse logs: {}", e)
                            )
                        ))?;
                    
                    // Convert EthLog to RawLog with error handling
                    let mut raw_logs = Vec::new();
                    for eth_log in eth_logs {
                        let block_number = parse_hex_to_u64(&eth_log.block_number)
                            .map_err(|e| IndexerError::Processing(
                                crate::error::ProcessingError::BlockParsing(
                                    format!("Invalid block number in log: {}", e)
                                )
                            ))?;
                        
                        let log_index = parse_hex_to_u32(&eth_log.log_index)
                            .map_err(|e| IndexerError::Processing(
                                crate::error::ProcessingError::LogParsing(
                                    format!("Invalid log index: {}", e)
                                )
                            ))?;
                        
                        raw_logs.push(RawLog {
                            address: eth_log.address,
                            topics: eth_log.topics,
                            data: eth_log.data,
                            block_number,
                            transaction_hash: eth_log.transaction_hash,
                            log_index,
                        });
                    }

                    let context = LogContext::new("rpc_client", "get_logs")
                        .with_metadata("log_count", serde_json::json!(raw_logs.len()))
                        .with_metadata("from_block", serde_json::json!(filter.from_block))
                        .with_metadata("to_block", serde_json::json!(filter.to_block));
                    context.debug(&format!("Retrieved {} logs", raw_logs.len()));

                    Ok(raw_logs)
                }
                Err(e) => Err(e),
            }
        }).await
    }
}

fn parse_hex_to_u64(hex_str: &str) -> Result<u64, RpcError> {
    let hex_without_prefix = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    u64::from_str_radix(hex_without_prefix, 16)
        .map_err(|e| RpcError::Rpc(format!("Failed to parse hex to u64: {}", e)))
}

fn parse_hex_to_u32(hex_str: &str) -> Result<u32, RpcError> {
    let hex_without_prefix = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    u32::from_str_radix(hex_without_prefix, 16)
        .map_err(|e| RpcError::Rpc(format!("Failed to parse hex to u32: {}", e)))
}

/// Enhanced hex parsing with better error handling
fn parse_hex_to_u64_enhanced(hex_str: &str) -> Result<u64, IndexerError> {
    let hex_without_prefix = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    u64::from_str_radix(hex_without_prefix, 16)
        .map_err(|e| IndexerError::Processing(
            crate::error::ProcessingError::AmountParsing(
                format!("Failed to parse hex '{}' to u64: {}", hex_str, e)
            )
        ))
}

fn parse_hex_to_u32_enhanced(hex_str: &str) -> Result<u32, IndexerError> {
    let hex_without_prefix = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    u32::from_str_radix(hex_without_prefix, 16)
        .map_err(|e| IndexerError::Processing(
            crate::error::ProcessingError::AmountParsing(
                format!("Failed to parse hex '{}' to u32: {}", hex_str, e)
            )
        ))
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

    #[test]
    fn test_parse_hex_to_u64() {
        assert_eq!(parse_hex_to_u64("0x1234").unwrap(), 0x1234u64);
        assert_eq!(parse_hex_to_u64("1234").unwrap(), 0x1234u64);
        assert_eq!(parse_hex_to_u64("0x0").unwrap(), 0u64);
        assert!(parse_hex_to_u64("invalid").is_err());
    }

    #[test]
    fn test_parse_hex_to_u32() {
        assert_eq!(parse_hex_to_u32("0x1234").unwrap(), 0x1234u32);
        assert_eq!(parse_hex_to_u32("1234").unwrap(), 0x1234u32);
        assert_eq!(parse_hex_to_u32("0x0").unwrap(), 0u32);
        assert!(parse_hex_to_u32("invalid").is_err());
    }

    #[test]
    fn test_log_filter_serialization() {
        let filter = LogFilter {
            from_block: "0x1234".to_string(),
            to_block: "0x1235".to_string(),
            address: Some("0xabc123".to_string()),
            topics: Some(vec![Some("0xdef456".to_string())]),
        };

        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("\"fromBlock\":\"0x1234\""));
        assert!(json.contains("\"toBlock\":\"0x1235\""));
        assert!(json.contains("\"address\":\"0xabc123\""));
    }

    // Mock server test would require additional dependencies like wiremock
    // For now, we'll test the parsing logic and structure
    #[test]
    fn test_rpc_error_display() {
        let rpc_error = RpcError::Rpc("Custom error".to_string());
        assert_eq!(format!("{}", rpc_error), "RPC error: Custom error");
    }
}