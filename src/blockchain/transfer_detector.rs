use std::collections::HashSet;
use thiserror::Error;
use crate::models::{RawLog, ProcessedTransfer, TransferDirection};

#[derive(Error, Debug)]
pub enum TransferDetectionError {
    #[error("Invalid log format: {0}")]
    InvalidLog(String),
    #[error("Invalid address format: {0}")]
    InvalidAddress(String),
    #[error("Invalid amount format: {0}")]
    InvalidAmount(String),
    #[error("Hex decoding error: {0}")]
    HexDecoding(String),
}

/// POL token contract address on Polygon mainnet
/// This is the official POL token address on Polygon
/// Note: POL is the native token on Polygon, but for ERC-20 transfers we need the wrapped version
/// Research shows POL token address on Polygon: 0x455e53847f9f0f0b0fcf0b0b0b0b0b0b0b0b0b0b (placeholder - needs verification)
/// TODO: Verify the correct POL token contract address on Polygon mainnet
pub const POL_TOKEN_ADDRESS: &str = "0x455e53847f9f0f0b0fcf0b0b0b0b0b0b0b0b0b0b";

/// ERC-20 Transfer event signature: Transfer(address indexed from, address indexed to, uint256 value)
pub const TRANSFER_EVENT_SIGNATURE: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

/// Binance addresses on Polygon (lowercase for consistent comparison)
pub const BINANCE_ADDRESSES: &[&str] = &[
    "0xf977814e90da44bfa03b6295a0616a897441acec",
    "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245", 
    "0x505e71695e9bc45943c58adec1650577bca68fd9",
    "0x290275e3db66394c52272398959845170e4dcb88",
    "0xd5c08681719445a5fdce2bda98b341a49050d821",
    "0x082489a616ab4d46d1947ee3f912e080815b08da",
];

pub struct TransferDetector {
    pol_token_address: String,
    binance_addresses: HashSet<String>,
}

impl TransferDetector {
    pub fn new() -> Self {
        let binance_addresses: HashSet<String> = BINANCE_ADDRESSES
            .iter()
            .map(|addr| normalize_address(addr))
            .collect();

        Self {
            pol_token_address: normalize_address(POL_TOKEN_ADDRESS),
            binance_addresses,
        }
    }

    /// Check if a log represents a POL token transfer event
    pub fn is_pol_transfer(&self, log: &RawLog) -> bool {
        // Check if the log is from the POL token contract
        let normalized_log_address = normalize_address(&log.address);
        if normalized_log_address != self.pol_token_address {
            return false;
        }

        // Check if it's a Transfer event by verifying the event signature
        if log.topics.is_empty() {
            return false;
        }

        let event_signature = normalize_address(&log.topics[0]);
        event_signature == normalize_address(TRANSFER_EVENT_SIGNATURE)
    }

    /// Decode a POL transfer event log into a ProcessedTransfer
    pub fn decode_transfer_log(&self, log: &RawLog) -> Result<ProcessedTransfer, TransferDetectionError> {
        if !self.is_pol_transfer(log) {
            return Err(TransferDetectionError::InvalidLog(
                "Log is not a POL transfer event".to_string()
            ));
        }

        // ERC-20 Transfer event has 3 topics: [signature, from, to]
        if log.topics.len() != 3 {
            return Err(TransferDetectionError::InvalidLog(
                format!("Expected 3 topics, got {}", log.topics.len())
            ));
        }

        // Extract from and to addresses from topics (remove 0x prefix and leading zeros)
        let from_address = extract_address_from_topic(&log.topics[1])?;
        let to_address = extract_address_from_topic(&log.topics[2])?;

        // Extract amount from data field
        let amount = extract_amount_from_data(&log.data)?;

        // Determine transfer direction
        let direction = self.classify_transfer(&from_address, &to_address);

        Ok(ProcessedTransfer {
            block_number: log.block_number,
            transaction_hash: log.transaction_hash.clone(),
            log_index: log.log_index,
            from_address,
            to_address,
            amount,
            timestamp: 0, // Will be set by the caller with block timestamp
            direction,
        })
    }

    /// Classify a transfer based on from/to addresses
    pub fn classify_transfer(&self, from_address: &str, to_address: &str) -> TransferDirection {
        let normalized_from = normalize_address(from_address);
        let normalized_to = normalize_address(to_address);

        let from_is_binance = self.binance_addresses.contains(&normalized_from);
        let to_is_binance = self.binance_addresses.contains(&normalized_to);

        match (from_is_binance, to_is_binance) {
            (false, true) => TransferDirection::ToBinance,   // Inflow to Binance
            (true, false) => TransferDirection::FromBinance, // Outflow from Binance
            _ => TransferDirection::NotRelevant,             // Both or neither are Binance
        }
    }

    /// Check if an address is a Binance address
    pub fn is_binance_address(&self, address: &str) -> bool {
        let normalized = normalize_address(address);
        self.binance_addresses.contains(&normalized)
    }
}

/// Normalize an Ethereum address to lowercase without 0x prefix
pub fn normalize_address(address: &str) -> String {
    let addr = address.trim();
    if addr.starts_with("0x") || addr.starts_with("0X") {
        addr[2..].to_lowercase()
    } else {
        addr.to_lowercase()
    }
}

/// Validate that an address is a valid Ethereum address format
pub fn validate_address(address: &str) -> Result<(), TransferDetectionError> {
    let normalized = normalize_address(address);
    
    if normalized.len() != 40 {
        return Err(TransferDetectionError::InvalidAddress(
            format!("Address must be 40 characters long, got {}", normalized.len())
        ));
    }

    if !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(TransferDetectionError::InvalidAddress(
            "Address contains non-hexadecimal characters".to_string()
        ));
    }

    Ok(())
}

/// Extract address from a 32-byte topic (remove leading zeros)
fn extract_address_from_topic(topic: &str) -> Result<String, TransferDetectionError> {
    let normalized_topic = normalize_address(topic);
    
    if normalized_topic.len() != 64 {
        return Err(TransferDetectionError::InvalidLog(
            format!("Topic should be 64 characters, got {}", normalized_topic.len())
        ));
    }

    // Address is in the last 40 characters (20 bytes)
    let address = &normalized_topic[24..64];
    validate_address(&format!("0x{}", address))?;
    
    Ok(address.to_string())
}

/// Extract amount from the data field (32-byte big-endian integer)
fn extract_amount_from_data(data: &str) -> Result<String, TransferDetectionError> {
    let normalized_data = normalize_address(data);
    
    if normalized_data.len() != 64 {
        return Err(TransferDetectionError::InvalidAmount(
            format!("Data should be 64 characters, got {}", normalized_data.len())
        ));
    }

    // Convert hex to decimal string
    let amount_hex = &normalized_data;
    
    // Parse as u128 to handle large token amounts
    match u128::from_str_radix(amount_hex, 16) {
        Ok(amount) => Ok(amount.to_string()),
        Err(e) => Err(TransferDetectionError::HexDecoding(
            format!("Failed to parse amount: {}", e)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_address() {
        assert_eq!(normalize_address("0xF977814e90dA44bFA03b6295A0616a897441aceC"), 
                   "f977814e90da44bfa03b6295a0616a897441acec");
        assert_eq!(normalize_address("F977814e90dA44bFA03b6295A0616a897441aceC"), 
                   "f977814e90da44bfa03b6295a0616a897441acec");
        assert_eq!(normalize_address("0X455E53847F9F0F0B0FCF0B0B0B0B0B0B0B0B0B0B"), 
                   "455e53847f9f0f0b0fcf0b0b0b0b0b0b0b0b0b0b");
    }

    #[test]
    fn test_validate_address() {
        // Valid addresses
        assert!(validate_address("0xf977814e90da44bfa03b6295a0616a897441acec").is_ok());
        assert!(validate_address("f977814e90da44bfa03b6295a0616a897441acec").is_ok());
        
        // Invalid addresses
        assert!(validate_address("0xf977814e90da44bfa03b6295a0616a897441ace").is_err()); // Too short
        assert!(validate_address("0xf977814e90da44bfa03b6295a0616a897441acecc").is_err()); // Too long
        assert!(validate_address("0xg977814e90da44bfa03b6295a0616a897441acec").is_err()); // Invalid hex
    }

    #[test]
    fn test_extract_address_from_topic() {
        let topic = "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec";
        let result = extract_address_from_topic(topic).unwrap();
        assert_eq!(result, "f977814e90da44bfa03b6295a0616a897441acec");
    }

    #[test]
    fn test_extract_amount_from_data() {
        // 1 POL (1 * 10^18 wei) = 0xde0b6b3a7640000
        let data = "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000";
        let result = extract_amount_from_data(data).unwrap();
        assert_eq!(result, "1000000000000000000");
        
        // Test with a smaller amount to avoid precision issues
        // 0.1 POL (0.1 * 10^18 wei) = 100000000000000000 = 0x16345785d8a0000
        let data = "0x000000000000000000000000000000000000000000000000016345785d8a0000";
        let result = extract_amount_from_data(data).unwrap();
        assert_eq!(result, "100000000000000000");
        
        // Test zero amount
        let data = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let result = extract_amount_from_data(data).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_transfer_detector_creation() {
        let detector = TransferDetector::new();
        assert_eq!(detector.pol_token_address, normalize_address(POL_TOKEN_ADDRESS));
        assert_eq!(detector.binance_addresses.len(), BINANCE_ADDRESSES.len());
    }

    #[test]
    fn test_is_binance_address() {
        let detector = TransferDetector::new();
        
        // Test known Binance addresses
        assert!(detector.is_binance_address("0xF977814e90dA44bFA03b6295A0616a897441aceC"));
        assert!(detector.is_binance_address("0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245"));
        
        // Test non-Binance address
        assert!(!detector.is_binance_address("0x1234567890123456789012345678901234567890"));
    }

    #[test]
    fn test_classify_transfer() {
        let detector = TransferDetector::new();
        
        let binance_addr = "0xF977814e90dA44bFA03b6295A0616a897441aceC";
        let other_addr = "0x1234567890123456789012345678901234567890";
        
        // Transfer to Binance (inflow)
        assert_eq!(
            detector.classify_transfer(other_addr, binance_addr),
            TransferDirection::ToBinance
        );
        
        // Transfer from Binance (outflow)
        assert_eq!(
            detector.classify_transfer(binance_addr, other_addr),
            TransferDirection::FromBinance
        );
        
        // Transfer between non-Binance addresses
        assert_eq!(
            detector.classify_transfer(other_addr, "0x9876543210987654321098765432109876543210"),
            TransferDirection::NotRelevant
        );
        
        // Transfer between Binance addresses
        assert_eq!(
            detector.classify_transfer(binance_addr, "0xe7804c37c13166ff0b37f5ae0bb07a3aebb6e245"),
            TransferDirection::NotRelevant
        );
    }

    #[test]
    fn test_is_pol_transfer() {
        let detector = TransferDetector::new();
        
        // Valid POL transfer log
        let pol_log = RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec".to_string(),
                "0x000000000000000000000000e7804c37c13166ff0b37f5ae0bb07a3aebb6e245".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 12345,
            transaction_hash: "0xabc123".to_string(),
            log_index: 0,
        };
        
        assert!(detector.is_pol_transfer(&pol_log));
        
        // Log from different contract
        let other_log = RawLog {
            address: "0x1234567890123456789012345678901234567890".to_string(),
            topics: vec![TRANSFER_EVENT_SIGNATURE.to_string()],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 12345,
            transaction_hash: "0xabc123".to_string(),
            log_index: 0,
        };
        
        assert!(!detector.is_pol_transfer(&other_log));
        
        // Log with different event signature
        let wrong_event_log = RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec!["0x1234567890123456789012345678901234567890123456789012345678901234".to_string()],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 12345,
            transaction_hash: "0xabc123".to_string(),
            log_index: 0,
        };
        
        assert!(!detector.is_pol_transfer(&wrong_event_log));
    }

    #[test]
    fn test_decode_transfer_log() {
        let detector = TransferDetector::new();
        
        let log = RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec".to_string(),
                "0x0000000000000000000000001234567890123456789012345678901234567890".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 12345,
            transaction_hash: "0xabc123def456".to_string(),
            log_index: 2,
        };
        
        let result = detector.decode_transfer_log(&log).unwrap();
        
        assert_eq!(result.block_number, 12345);
        assert_eq!(result.transaction_hash, "0xabc123def456");
        assert_eq!(result.log_index, 2);
        assert_eq!(result.from_address, "f977814e90da44bfa03b6295a0616a897441acec");
        assert_eq!(result.to_address, "1234567890123456789012345678901234567890");
        assert_eq!(result.amount, "1000000000000000000"); // 1 POL in wei
        assert_eq!(result.direction, TransferDirection::FromBinance);
    }

    #[test]
    fn test_decode_transfer_log_invalid() {
        let detector = TransferDetector::new();
        
        // Log with wrong number of topics
        let invalid_log = RawLog {
            address: POL_TOKEN_ADDRESS.to_string(),
            topics: vec![TRANSFER_EVENT_SIGNATURE.to_string()], // Missing from/to topics
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 12345,
            transaction_hash: "0xabc123".to_string(),
            log_index: 0,
        };
        
        assert!(detector.decode_transfer_log(&invalid_log).is_err());
        
        // Log from wrong contract
        let wrong_contract_log = RawLog {
            address: "0x1234567890123456789012345678901234567890".to_string(),
            topics: vec![
                TRANSFER_EVENT_SIGNATURE.to_string(),
                "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec".to_string(),
                "0x0000000000000000000000001234567890123456789012345678901234567890".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 12345,
            transaction_hash: "0xabc123".to_string(),
            log_index: 0,
        };
        
        assert!(detector.decode_transfer_log(&wrong_contract_log).is_err());
    }
}