use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessedTransfer {
    pub block_number: u64,
    pub transaction_hash: String,
    pub log_index: u32,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,  // Decimal string representation for precision
    pub timestamp: u64,
    pub direction: TransferDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    pub block_number: u64,
    pub transaction_hash: String,
    pub log_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferDirection {
    ToBinance,    // Inflow to Binance
    FromBinance,  // Outflow from Binance
    NotRelevant,  // Transfer not involving Binance
}
#
[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_processed_transfer_serialization() {
        let transfer = ProcessedTransfer {
            block_number: 12345,
            transaction_hash: "0xabc123".to_string(),
            log_index: 0,
            from_address: "0x1234567890abcdef".to_string(),
            to_address: "0xfedcba0987654321".to_string(),
            amount: "1000000000000000000".to_string(), // 1 POL in wei
            timestamp: 1640995200,
            direction: TransferDirection::ToBinance,
        };

        // Test serialization
        let json = serde_json::to_string(&transfer).expect("Failed to serialize");
        assert!(json.contains("\"block_number\":12345"));
        assert!(json.contains("\"direction\":\"ToBinance\""));

        // Test deserialization
        let deserialized: ProcessedTransfer = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(transfer, deserialized);
    }

    #[test]
    fn test_raw_log_serialization() {
        let raw_log = RawLog {
            address: "0x455e53847f9f0f0b0fcf0b0b0b0b0b0b0b0b0b0b".to_string(),
            topics: vec![
                "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".to_string(),
                "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000".to_string(),
            block_number: 54321,
            transaction_hash: "0xdef456".to_string(),
            log_index: 2,
        };

        // Test serialization
        let json = serde_json::to_string(&raw_log).expect("Failed to serialize");
        assert!(json.contains("\"block_number\":54321"));
        assert!(json.contains("\"log_index\":2"));

        // Test deserialization
        let deserialized: RawLog = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(raw_log, deserialized);
    }

    #[test]
    fn test_transfer_direction_serialization() {
        // Test ToBinance
        let to_binance = TransferDirection::ToBinance;
        let json = serde_json::to_string(&to_binance).expect("Failed to serialize");
        assert_eq!(json, "\"ToBinance\"");
        let deserialized: TransferDirection = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(to_binance, deserialized);

        // Test FromBinance
        let from_binance = TransferDirection::FromBinance;
        let json = serde_json::to_string(&from_binance).expect("Failed to serialize");
        assert_eq!(json, "\"FromBinance\"");
        let deserialized: TransferDirection = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(from_binance, deserialized);

        // Test NotRelevant
        let not_relevant = TransferDirection::NotRelevant;
        let json = serde_json::to_string(&not_relevant).expect("Failed to serialize");
        assert_eq!(json, "\"NotRelevant\"");
        let deserialized: TransferDirection = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(not_relevant, deserialized);
    }

    #[test]
    fn test_processed_transfer_with_different_directions() {
        let directions = vec![
            TransferDirection::ToBinance,
            TransferDirection::FromBinance,
            TransferDirection::NotRelevant,
        ];

        for direction in directions {
            let transfer = ProcessedTransfer {
                block_number: 1,
                transaction_hash: "0x123".to_string(),
                log_index: 0,
                from_address: "0x111".to_string(),
                to_address: "0x222".to_string(),
                amount: "100".to_string(),
                timestamp: 1640995200,
                direction: direction.clone(),
            };

            let json = serde_json::to_string(&transfer).expect("Failed to serialize");
            let deserialized: ProcessedTransfer = serde_json::from_str(&json).expect("Failed to deserialize");
            assert_eq!(transfer.direction, deserialized.direction);
        }
    }
}