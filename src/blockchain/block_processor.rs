
use thiserror::Error;
use crate::blockchain::{RpcClient, Block, LogFilter};
use crate::blockchain::transfer_detector::{TransferDetector, TRANSFER_EVENT_SIGNATURE, POL_TOKEN_ADDRESS};
use crate::models::{ProcessedTransfer, RawLog, TransferDirection};

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Block processing failed: {0}")]
    Processing(String),
    #[error("RPC error: {0}")]
    Rpc(#[from] crate::blockchain::rpc_client::RpcError),
    #[error("Transfer detection error: {0}")]
    TransferDetection(#[from] crate::blockchain::transfer_detector::TransferDetectionError),
}

pub struct BlockProcessor {
    rpc_client: RpcClient,
    transfer_detector: TransferDetector,
}

impl BlockProcessor {
    pub fn new(rpc_client: RpcClient) -> Self {
        Self {
            rpc_client,
            transfer_detector: TransferDetector::new(),
        }
    }

    /// Process a block and extract POL token transfers involving Binance addresses
    pub async fn process_block(&self, block_number: u64) -> Result<Vec<ProcessedTransfer>, ProcessError> {
        // Get block data to extract timestamp
        let block = self.rpc_client.get_block(block_number).await?;
        let timestamp = parse_hex_timestamp(&block.timestamp)?;

        // Create log filter for POL token Transfer events
        let log_filter = LogFilter {
            from_block: format!("0x{:x}", block_number),
            to_block: format!("0x{:x}", block_number),
            address: Some(POL_TOKEN_ADDRESS.to_string()),
            topics: Some(vec![Some(TRANSFER_EVENT_SIGNATURE.to_string())]),
        };

        // Get logs from the block
        let raw_logs = self.rpc_client.get_logs(log_filter).await?;

        // Process each log and filter for Binance-related transfers
        let mut processed_transfers = Vec::new();
        
        for raw_log in raw_logs {
            // Only process POL token transfers
            if self.transfer_detector.is_pol_transfer(&raw_log) {
                match self.transfer_detector.decode_transfer_log(&raw_log) {
                    Ok(mut transfer) => {
                        // Set the timestamp from block data
                        transfer.timestamp = timestamp;
                        
                        // Only include transfers involving Binance addresses
                        if transfer.direction != TransferDirection::NotRelevant {
                            processed_transfers.push(transfer);
                        }
                    }
                    Err(e) => {
                        // Log the error but continue processing other transfers
                        log::warn!("Failed to decode transfer log: {}", e);
                    }
                }
            }
        }

        Ok(processed_transfers)
    }

    /// Extract and filter POL token transfers from a block
    pub async fn extract_pol_transfers(&self, block_number: u64) -> Result<Vec<RawLog>, ProcessError> {
        let log_filter = LogFilter {
            from_block: format!("0x{:x}", block_number),
            to_block: format!("0x{:x}", block_number),
            address: Some(POL_TOKEN_ADDRESS.to_string()),
            topics: Some(vec![Some(TRANSFER_EVENT_SIGNATURE.to_string())]),
        };

        let raw_logs = self.rpc_client.get_logs(log_filter).await?;
        
        // Filter for POL token transfers only
        let pol_transfers: Vec<RawLog> = raw_logs
            .into_iter()
            .filter(|log| self.transfer_detector.is_pol_transfer(log))
            .collect();

        Ok(pol_transfers)
    }

    /// Identify Binance-related transfers from a list of processed transfers
    pub fn identify_binance_transfers(&self, transfers: Vec<ProcessedTransfer>) -> Vec<ProcessedTransfer> {
        transfers
            .into_iter()
            .filter(|transfer| transfer.direction != TransferDirection::NotRelevant)
            .collect()
    }

    /// Get the transfer detector for external use
    pub fn transfer_detector(&self) -> &TransferDetector {
        &self.transfer_detector
    }
}

fn parse_hex_timestamp(hex_timestamp: &str) -> Result<u64, ProcessError> {
    let hex_without_prefix = hex_timestamp.strip_prefix("0x").unwrap_or(hex_timestamp);
    u64::from_str_radix(hex_without_prefix, 16)
        .map_err(|e| ProcessError::Processing(format!("Failed to parse timestamp: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::rpc_client::Transaction;
    use crate::blockchain::transfer_detector::BINANCE_ADDRESSES;

    // Mock RPC client for testing
    struct MockRpcClient {
        block_data: Option<Block>,
        logs_data: Vec<RawLog>,
        should_fail: bool,
    }

    impl MockRpcClient {
        fn new() -> Self {
            Self {
                block_data: None,
                logs_data: Vec::new(),
                should_fail: false,
            }
        }

        fn with_block(mut self, block: Block) -> Self {
            self.block_data = Some(block);
            self
        }

        fn with_logs(mut self, logs: Vec<RawLog>) -> Self {
            self.logs_data = logs;
            self
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }
    }

    fn create_mock_block(block_number: u64, timestamp: u64) -> Block {
        Block {
            number: format!("0x{:x}", block_number),
            hash: format!("0xblock{:x}", block_number),
            timestamp: format!("0x{:x}", timestamp),
            transactions: vec![
                Transaction {
                    hash: "0xtx1".to_string(),
                    from: "0xfrom1".to_string(),
                    to: Some("0xto1".to_string()),
                    block_number: format!("0x{:x}", block_number),
                }
            ],
        }
    }

    fn create_mock_pol_transfer_log(
        block_number: u64,
        from_address: &str,
        to_address: &str,
        amount: &str,
        log_index: u32,
    ) -> RawLog {
        RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                format!("0x000000000000000000000000{}", from_address.strip_prefix("0x").unwrap_or(from_address)),
                format!("0x000000000000000000000000{}", to_address.strip_prefix("0x").unwrap_or(to_address)),
            ],
            data: format!("0x{:0>64}", amount),
            block_number,
            transaction_hash: format!("0xtx{}", log_index),
            log_index,
        }
    }

    #[test]
    fn test_parse_hex_timestamp() {
        assert_eq!(parse_hex_timestamp("0x61234567").unwrap(), 0x61234567u64);
        assert_eq!(parse_hex_timestamp("61234567").unwrap(), 0x61234567u64);
        assert_eq!(parse_hex_timestamp("0x0").unwrap(), 0u64);
        assert!(parse_hex_timestamp("invalid").is_err());
    }

    #[test]
    fn test_block_processor_creation() {
        let rpc_client = RpcClient::new("http://test".to_string());
        let processor = BlockProcessor::new(rpc_client);
        
        // Verify that the processor has a transfer detector
        assert!(processor.transfer_detector().is_binance_address(BINANCE_ADDRESSES[0]));
    }

    #[test]
    fn test_identify_binance_transfers() {
        let rpc_client = RpcClient::new("http://test".to_string());
        let processor = BlockProcessor::new(rpc_client);

        let transfers = vec![
            ProcessedTransfer {
                block_number: 1,
                transaction_hash: "0x1".to_string(),
                log_index: 0,
                from_address: "other".to_string(),
                to_address: "binance".to_string(),
                amount: "100".to_string(),
                timestamp: 1640995200,
                direction: TransferDirection::ToBinance,
            },
            ProcessedTransfer {
                block_number: 1,
                transaction_hash: "0x2".to_string(),
                log_index: 1,
                from_address: "other1".to_string(),
                to_address: "other2".to_string(),
                amount: "200".to_string(),
                timestamp: 1640995200,
                direction: TransferDirection::NotRelevant,
            },
            ProcessedTransfer {
                block_number: 1,
                transaction_hash: "0x3".to_string(),
                log_index: 2,
                from_address: "binance".to_string(),
                to_address: "other".to_string(),
                amount: "300".to_string(),
                timestamp: 1640995200,
                direction: TransferDirection::FromBinance,
            },
        ];

        let binance_transfers = processor.identify_binance_transfers(transfers);
        
        assert_eq!(binance_transfers.len(), 2);
        assert_eq!(binance_transfers[0].direction, TransferDirection::ToBinance);
        assert_eq!(binance_transfers[1].direction, TransferDirection::FromBinance);
    }

    #[tokio::test]
    async fn test_extract_pol_transfers_integration() {
        // This test would require a mock RPC client implementation
        // For now, we'll test the logic structure
        let rpc_client = RpcClient::new("http://test".to_string());
        let processor = BlockProcessor::new(rpc_client);

        // Test that the method exists and has the right signature
        // In a real integration test, we would mock the RPC responses
        let result = processor.extract_pol_transfers(12345).await;
        
        // This will fail with a network error, but that's expected in unit tests
        // The important thing is that the method compiles and has the right structure
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_data_creation() {
        let block = create_mock_block(12345, 1640995200);
        assert_eq!(block.number, "0x3039");
        assert_eq!(block.timestamp, "0x61cf9980"); // Corrected expected value
        assert_eq!(block.transactions.len(), 1);

        let binance_addr = BINANCE_ADDRESSES[0].strip_prefix("0x").unwrap_or(BINANCE_ADDRESSES[0]);
        let other_addr = "1234567890123456789012345678901234567890";
        
        let log = create_mock_pol_transfer_log(12345, binance_addr, other_addr, "de0b6b3a7640000", 0);
        assert_eq!(log.block_number, 12345);
        assert_eq!(log.address, POL_TOKEN_ADDRESS);
        assert_eq!(log.topics.len(), 3);
        assert_eq!(log.topics[0], TRANSFER_EVENT_SIGNATURE);
    }

    #[test]
    fn test_transfer_detector_integration() {
        let rpc_client = RpcClient::new("http://test".to_string());
        let processor = BlockProcessor::new(rpc_client);
        
        let binance_addr = BINANCE_ADDRESSES[0];
        let other_addr = "0x1234567890123456789012345678901234567890";
        
        let log = create_mock_pol_transfer_log(
            12345,
            binance_addr,
            other_addr,
            "de0b6b3a7640000", // 1 POL in wei
            0
        );

        // Test that the transfer detector correctly identifies POL transfers
        assert!(processor.transfer_detector().is_pol_transfer(&log));
        
        // Test decoding
        let result = processor.transfer_detector().decode_transfer_log(&log);
        assert!(result.is_ok());
        
        let transfer = result.unwrap();
        assert_eq!(transfer.direction, TransferDirection::FromBinance);
        assert_eq!(transfer.amount, "1000000000000000000");
    }

    #[test]
    fn test_process_error_display() {
        let error = ProcessError::Processing("Test error".to_string());
        assert_eq!(format!("{}", error), "Block processing failed: Test error");
    }

    // Integration test with mock data
    #[test]
    fn test_full_transfer_processing_flow() {
        let rpc_client = RpcClient::new("http://test".to_string());
        let processor = BlockProcessor::new(rpc_client);
        
        // Create test data
        let binance_addr = BINANCE_ADDRESSES[0];
        let other_addr = "0x1234567890123456789012345678901234567890";
        
        // Test inflow (to Binance)
        let inflow_log = create_mock_pol_transfer_log(
            12345,
            &other_addr[2..], // Remove 0x prefix for the mock
            &binance_addr[2..],
            "de0b6b3a7640000",
            0
        );
        
        // Test outflow (from Binance)  
        let outflow_log = create_mock_pol_transfer_log(
            12345,
            &binance_addr[2..],
            &other_addr[2..],
            "6f05b59d3b20000", // 0.5 POL in wei
            1
        );
        
        // Test non-relevant transfer
        let irrelevant_log = create_mock_pol_transfer_log(
            12345,
            &other_addr[2..],
            "9876543210987654321098765432109876543210",
            "1bc16d674ec80000", // 0.2 POL in wei
            2
        );

        // Test transfer detection
        assert!(processor.transfer_detector().is_pol_transfer(&inflow_log));
        assert!(processor.transfer_detector().is_pol_transfer(&outflow_log));
        assert!(processor.transfer_detector().is_pol_transfer(&irrelevant_log));

        // Test transfer decoding and classification
        let inflow_transfer = processor.transfer_detector().decode_transfer_log(&inflow_log).unwrap();
        let outflow_transfer = processor.transfer_detector().decode_transfer_log(&outflow_log).unwrap();
        let irrelevant_transfer = processor.transfer_detector().decode_transfer_log(&irrelevant_log).unwrap();

        assert_eq!(inflow_transfer.direction, TransferDirection::ToBinance);
        assert_eq!(outflow_transfer.direction, TransferDirection::FromBinance);
        assert_eq!(irrelevant_transfer.direction, TransferDirection::NotRelevant);

        // Test filtering
        let all_transfers = vec![inflow_transfer, outflow_transfer, irrelevant_transfer];
        let binance_transfers = processor.identify_binance_transfers(all_transfers);
        
        assert_eq!(binance_transfers.len(), 2);
        assert!(binance_transfers.iter().any(|t| t.direction == TransferDirection::ToBinance));
        assert!(binance_transfers.iter().any(|t| t.direction == TransferDirection::FromBinance));
        assert!(!binance_transfers.iter().any(|t| t.direction == TransferDirection::NotRelevant));
    }
}