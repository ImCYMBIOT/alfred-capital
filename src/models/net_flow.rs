use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetFlowData {
    pub total_inflow: String,   // Decimal string for precision
    pub total_outflow: String,  // Decimal string for precision
    pub net_flow: String,       // Can be negative (inflow - outflow)
    pub last_processed_block: u64,
    pub last_updated: u64,
}

impl Default for NetFlowData {
    fn default() -> Self {
        Self {
            total_inflow: "0".to_string(),
            total_outflow: "0".to_string(),
            net_flow: "0".to_string(),
            last_processed_block: 0,
            last_updated: 0,
        }
    }
}

pub struct NetFlowCalculator;

impl NetFlowCalculator {
    /// Add an inflow amount to the current total inflow
    pub fn add_inflow(current: &str, amount: &str) -> Result<String, CalculationError> {
        let current_val = Self::parse_decimal(current)?;
        let amount_val = Self::parse_decimal(amount)?;
        Ok((current_val + amount_val).to_string())
    }

    /// Add an outflow amount to the current total outflow
    pub fn add_outflow(current: &str, amount: &str) -> Result<String, CalculationError> {
        let current_val = Self::parse_decimal(current)?;
        let amount_val = Self::parse_decimal(amount)?;
        Ok((current_val + amount_val).to_string())
    }

    /// Calculate net flow (inflow - outflow)
    pub fn calculate_net(inflow: &str, outflow: &str) -> Result<String, CalculationError> {
        let inflow_val = Self::parse_decimal(inflow)?;
        let outflow_val = Self::parse_decimal(outflow)?;
        Ok((inflow_val - outflow_val).to_string())
    }

    /// Parse decimal string to f64 for calculations
    /// Note: In production, consider using a decimal library for exact precision
    fn parse_decimal(value: &str) -> Result<f64, CalculationError> {
        f64::from_str(value).map_err(|_| CalculationError::InvalidDecimal(value.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CalculationError {
    #[error("Invalid decimal format: {0}")]
    InvalidDecimal(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_net_flow_data_serialization() {
        let net_flow = NetFlowData {
            total_inflow: "1500000000000000000000".to_string(), // 1500 POL
            total_outflow: "500000000000000000000".to_string(),  // 500 POL
            net_flow: "1000000000000000000000".to_string(),     // 1000 POL net
            last_processed_block: 98765,
            last_updated: 1640995200,
        };

        // Test serialization
        let json = serde_json::to_string(&net_flow).expect("Failed to serialize");
        assert!(json.contains("\"total_inflow\":\"1500000000000000000000\""));
        assert!(json.contains("\"last_processed_block\":98765"));

        // Test deserialization
        let deserialized: NetFlowData = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(net_flow, deserialized);
    }

    #[test]
    fn test_net_flow_data_default() {
        let default_flow = NetFlowData::default();
        assert_eq!(default_flow.total_inflow, "0");
        assert_eq!(default_flow.total_outflow, "0");
        assert_eq!(default_flow.net_flow, "0");
        assert_eq!(default_flow.last_processed_block, 0);
        assert_eq!(default_flow.last_updated, 0);
    }

    #[test]
    fn test_net_flow_calculator_add_inflow() {
        // Test adding to zero
        let result = NetFlowCalculator::add_inflow("0", "1000").expect("Failed to add inflow");
        assert_eq!(result, "1000");

        // Test adding to existing amount
        let result = NetFlowCalculator::add_inflow("1000", "500").expect("Failed to add inflow");
        assert_eq!(result, "1500");

        // Test with decimal amounts
        let result = NetFlowCalculator::add_inflow("1000.5", "500.25").expect("Failed to add inflow");
        assert_eq!(result, "1500.75");
    }

    #[test]
    fn test_net_flow_calculator_add_outflow() {
        // Test adding to zero
        let result = NetFlowCalculator::add_outflow("0", "750").expect("Failed to add outflow");
        assert_eq!(result, "750");

        // Test adding to existing amount
        let result = NetFlowCalculator::add_outflow("750", "250").expect("Failed to add outflow");
        assert_eq!(result, "1000");

        // Test with decimal amounts
        let result = NetFlowCalculator::add_outflow("750.75", "249.25").expect("Failed to add outflow");
        assert_eq!(result, "1000");
    }

    #[test]
    fn test_net_flow_calculator_calculate_net() {
        // Test positive net flow (more inflow than outflow)
        let result = NetFlowCalculator::calculate_net("1500", "500").expect("Failed to calculate net");
        assert_eq!(result, "1000");

        // Test negative net flow (more outflow than inflow)
        let result = NetFlowCalculator::calculate_net("500", "1500").expect("Failed to calculate net");
        assert_eq!(result, "-1000");

        // Test zero net flow
        let result = NetFlowCalculator::calculate_net("1000", "1000").expect("Failed to calculate net");
        assert_eq!(result, "0");

        // Test with decimals
        let result = NetFlowCalculator::calculate_net("1000.75", "500.25").expect("Failed to calculate net");
        assert_eq!(result, "500.5");
    }

    #[test]
    fn test_net_flow_calculator_invalid_input() {
        // Test invalid inflow input
        let result = NetFlowCalculator::add_inflow("invalid", "100");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CalculationError::InvalidDecimal(_)));

        // Test invalid outflow input
        let result = NetFlowCalculator::add_outflow("100", "invalid");
        assert!(result.is_err());

        // Test invalid net calculation input
        let result = NetFlowCalculator::calculate_net("invalid", "100");
        assert!(result.is_err());
    }

    #[test]
    fn test_calculation_error_display() {
        let error = CalculationError::InvalidDecimal("not_a_number".to_string());
        let error_string = format!("{}", error);
        assert!(error_string.contains("Invalid decimal format: not_a_number"));
    }

    #[test]
    fn test_net_flow_data_with_large_numbers() {
        // Test with very large numbers (simulating real POL amounts in wei)
        let net_flow = NetFlowData {
            total_inflow: "1000000000000000000000000".to_string(), // 1M POL in wei
            total_outflow: "500000000000000000000000".to_string(),  // 500K POL in wei
            net_flow: "500000000000000000000000".to_string(),      // 500K POL net
            last_processed_block: 999999,
            last_updated: 1640995200,
        };

        let json = serde_json::to_string(&net_flow).expect("Failed to serialize large numbers");
        let deserialized: NetFlowData = serde_json::from_str(&json).expect("Failed to deserialize large numbers");
        assert_eq!(net_flow, deserialized);
    }
}