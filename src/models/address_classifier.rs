use std::collections::HashSet;
use once_cell::sync::Lazy;
use crate::models::TransferDirection;

/// Binance addresses in lowercase format for case-insensitive comparison
/// Based on requirement 7.1: Use the provided list of Binance addresses
pub const BINANCE_ADDRESSES: &[&str] = &[
    "f977814e90da44bfa03b6295a0616a897441acec",
    "e7804c37c13166ff0b37f5ae0bb07a3aebb6e245", 
    "505e71695e9bc45943c58adec1650577bca68fd9",
    "290275e3db66394c52272398959845170e4dcb88",
    "d5c08681719445a5fdce2bda98b341a49050d821",
    "082489a616ab4d46d1947ee3f912e080815b08da",
];

/// Pre-computed HashSet for O(1) address lookups
static BINANCE_ADDRESS_SET: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    BINANCE_ADDRESSES.iter().copied().collect()
});

/// Address classifier for determining if addresses are Binance-related
pub struct AddressClassifier;

impl AddressClassifier {
    /// Check if an address is a Binance address
    /// Performs case-insensitive comparison by converting to lowercase
    pub fn is_binance_address(address: &str) -> bool {
        let normalized_address = Self::normalize_address(address);
        BINANCE_ADDRESS_SET.contains(normalized_address.as_str())
    }

    /// Classify a transfer based on from/to addresses
    /// Returns the direction of the transfer relative to Binance
    /// 
    /// Requirements addressed:
    /// - 7.2: Correctly identify transfers TO Binance (inflows)
    /// - 7.3: Correctly identify transfers FROM Binance (outflows)
    pub fn classify_transfer(from_address: &str, to_address: &str) -> TransferDirection {
        let from_is_binance = Self::is_binance_address(from_address);
        let to_is_binance = Self::is_binance_address(to_address);

        match (from_is_binance, to_is_binance) {
            (false, true) => TransferDirection::ToBinance,   // Inflow to Binance
            (true, false) => TransferDirection::FromBinance, // Outflow from Binance
            _ => TransferDirection::NotRelevant,             // Both or neither are Binance
        }
    }

    /// Normalize address to lowercase for consistent comparison
    /// Removes 0x prefix if present and converts to lowercase
    fn normalize_address(address: &str) -> String {
        let trimmed = address.trim();
        let without_prefix = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
            &trimmed[2..]
        } else {
            trimmed
        };
        without_prefix.to_lowercase()
    }

    /// Get all Binance addresses as a vector (for testing/debugging)
    pub fn get_binance_addresses() -> Vec<&'static str> {
        BINANCE_ADDRESSES.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_address_detection() {
        // Test with exact addresses from the list
        assert!(AddressClassifier::is_binance_address("0xF977814e90dA44bFA03b6295A0616a897441aceC"));
        assert!(AddressClassifier::is_binance_address("0xe7804c37c13166fF0b37F5aE0BB07A3aEbb6e245"));
        assert!(AddressClassifier::is_binance_address("0x505e71695E9bc45943c58adEC1650577BcA68fD9"));
        
        // Test case insensitivity
        assert!(AddressClassifier::is_binance_address("0XF977814E90DA44BFA03B6295A0616A897441ACEC"));
        assert!(AddressClassifier::is_binance_address("0xf977814e90da44bfa03b6295a0616a897441acec"));
        
        // Test without 0x prefix
        assert!(AddressClassifier::is_binance_address("f977814e90da44bfa03b6295a0616a897441acec"));
        assert!(AddressClassifier::is_binance_address("F977814E90DA44BFA03B6295A0616A897441ACEC"));
        
        // Test non-Binance addresses
        assert!(!AddressClassifier::is_binance_address("0x1234567890abcdef1234567890abcdef12345678"));
        assert!(!AddressClassifier::is_binance_address("0x0000000000000000000000000000000000000000"));
        assert!(!AddressClassifier::is_binance_address(""));
    }

    #[test]
    fn test_address_normalization() {
        // Test with 0x prefix
        assert_eq!(
            AddressClassifier::normalize_address("0xF977814e90dA44bFA03b6295A0616a897441aceC"),
            "f977814e90da44bfa03b6295a0616a897441acec"
        );
        
        // Test with 0X prefix
        assert_eq!(
            AddressClassifier::normalize_address("0XF977814E90DA44BFA03B6295A0616A897441ACEC"),
            "f977814e90da44bfa03b6295a0616a897441acec"
        );
        
        // Test without prefix
        assert_eq!(
            AddressClassifier::normalize_address("F977814E90DA44BFA03B6295A0616A897441ACEC"),
            "f977814e90da44bfa03b6295a0616a897441acec"
        );
        
        // Test with whitespace
        assert_eq!(
            AddressClassifier::normalize_address("  0xF977814e90dA44bFA03b6295A0616a897441aceC  "),
            "f977814e90da44bfa03b6295a0616a897441acec"
        );
    }

    #[test]
    fn test_transfer_classification_to_binance() {
        let binance_addr = "0xF977814e90dA44bFA03b6295A0616a897441aceC";
        let regular_addr = "0x1234567890abcdef1234567890abcdef12345678";
        
        // Transfer TO Binance (inflow)
        let direction = AddressClassifier::classify_transfer(regular_addr, binance_addr);
        assert_eq!(direction, TransferDirection::ToBinance);
    }

    #[test]
    fn test_transfer_classification_from_binance() {
        let binance_addr = "0xe7804c37c13166fF0b37F5aE0BB07A3aEbb6e245";
        let regular_addr = "0x9876543210fedcba9876543210fedcba98765432";
        
        // Transfer FROM Binance (outflow)
        let direction = AddressClassifier::classify_transfer(binance_addr, regular_addr);
        assert_eq!(direction, TransferDirection::FromBinance);
    }

    #[test]
    fn test_transfer_classification_not_relevant() {
        let regular_addr1 = "0x1234567890abcdef1234567890abcdef12345678";
        let regular_addr2 = "0x9876543210fedcba9876543210fedcba98765432";
        let binance_addr1 = "0xF977814e90dA44bFA03b6295A0616a897441aceC";
        let binance_addr2 = "0xe7804c37c13166fF0b37F5aE0BB07A3aEbb6e245";
        
        // Transfer between regular addresses (not relevant)
        let direction = AddressClassifier::classify_transfer(regular_addr1, regular_addr2);
        assert_eq!(direction, TransferDirection::NotRelevant);
        
        // Transfer between Binance addresses (not relevant for net flow)
        let direction = AddressClassifier::classify_transfer(binance_addr1, binance_addr2);
        assert_eq!(direction, TransferDirection::NotRelevant);
    }

    #[test]
    fn test_all_binance_addresses_recognized() {
        // Test that all addresses in the constant array are recognized
        for &address in BINANCE_ADDRESSES {
            assert!(AddressClassifier::is_binance_address(address));
            assert!(AddressClassifier::is_binance_address(&format!("0x{}", address)));
            assert!(AddressClassifier::is_binance_address(&address.to_uppercase()));
            assert!(AddressClassifier::is_binance_address(&format!("0x{}", address.to_uppercase())));
        }
    }

    #[test]
    fn test_case_insensitive_classification() {
        let binance_addr_lower = "0xf977814e90da44bfa03b6295a0616a897441acec";
        let binance_addr_upper = "0XF977814E90DA44BFA03B6295A0616A897441ACEC";
        let binance_addr_mixed = "0xF977814e90dA44bFA03b6295A0616a897441aceC";
        let regular_addr = "0x1234567890abcdef1234567890abcdef12345678";
        
        // All should be classified the same way regardless of case
        assert_eq!(
            AddressClassifier::classify_transfer(regular_addr, binance_addr_lower),
            TransferDirection::ToBinance
        );
        assert_eq!(
            AddressClassifier::classify_transfer(regular_addr, binance_addr_upper),
            TransferDirection::ToBinance
        );
        assert_eq!(
            AddressClassifier::classify_transfer(regular_addr, binance_addr_mixed),
            TransferDirection::ToBinance
        );
        
        assert_eq!(
            AddressClassifier::classify_transfer(binance_addr_lower, regular_addr),
            TransferDirection::FromBinance
        );
        assert_eq!(
            AddressClassifier::classify_transfer(binance_addr_upper, regular_addr),
            TransferDirection::FromBinance
        );
        assert_eq!(
            AddressClassifier::classify_transfer(binance_addr_mixed, regular_addr),
            TransferDirection::FromBinance
        );
    }

    #[test]
    fn test_get_binance_addresses() {
        let addresses = AddressClassifier::get_binance_addresses();
        assert_eq!(addresses.len(), 6);
        assert!(addresses.contains(&"f977814e90da44bfa03b6295a0616a897441acec"));
        assert!(addresses.contains(&"e7804c37c13166ff0b37f5ae0bb07a3aebb6e245"));
        assert!(addresses.contains(&"505e71695e9bc45943c58adec1650577bca68fd9"));
        assert!(addresses.contains(&"290275e3db66394c52272398959845170e4dcb88"));
        assert!(addresses.contains(&"d5c08681719445a5fdce2bda98b341a49050d821"));
        assert!(addresses.contains(&"082489a616ab4d46d1947ee3f912e080815b08da"));
    }

    #[test]
    fn test_edge_cases() {
        // Test empty strings
        assert!(!AddressClassifier::is_binance_address(""));
        assert_eq!(
            AddressClassifier::classify_transfer("", ""),
            TransferDirection::NotRelevant
        );
        
        // Test malformed addresses (too short)
        assert!(!AddressClassifier::is_binance_address("0x123"));
        
        // Test self-transfer with Binance address
        let binance_addr = "0xF977814e90dA44bFA03b6295A0616a897441aceC";
        assert_eq!(
            AddressClassifier::classify_transfer(binance_addr, binance_addr),
            TransferDirection::NotRelevant
        );
    }
}