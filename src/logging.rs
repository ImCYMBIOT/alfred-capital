use log::{info, warn, error, debug, trace};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Structured logging context for the indexer
pub struct LogContext {
    pub component: String,
    pub operation: String,
    pub metadata: HashMap<String, Value>,
}

impl LogContext {
    pub fn new(component: &str, operation: &str) -> Self {
        Self {
            component: component.to_string(),
            operation: operation.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    pub fn with_block_number(self, block_number: u64) -> Self {
        self.with_metadata("block_number", json!(block_number))
    }

    pub fn with_transaction_hash(self, tx_hash: &str) -> Self {
        self.with_metadata("transaction_hash", json!(tx_hash))
    }

    pub fn with_address(self, address: &str) -> Self {
        self.with_metadata("address", json!(address))
    }

    pub fn with_amount(self, amount: &str) -> Self {
        self.with_metadata("amount", json!(amount))
    }

    pub fn with_duration_ms(self, duration_ms: u64) -> Self {
        self.with_metadata("duration_ms", json!(duration_ms))
    }

    pub fn with_retry_count(self, retry_count: u32) -> Self {
        self.with_metadata("retry_count", json!(retry_count))
    }

    pub fn with_error_code(self, error_code: &str) -> Self {
        self.with_metadata("error_code", json!(error_code))
    }

    fn format_message(&self, level: &str, message: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut log_entry = json!({
            "timestamp": timestamp,
            "level": level,
            "component": self.component,
            "operation": self.operation,
            "message": message,
        });

        // Add metadata
        for (key, value) in &self.metadata {
            log_entry[key] = value.clone();
        }

        log_entry.to_string()
    }

    pub fn info(&self, message: &str) {
        info!("{}", self.format_message("INFO", message));
    }

    pub fn warn(&self, message: &str) {
        warn!("{}", self.format_message("WARN", message));
    }

    pub fn error(&self, message: &str) {
        error!("{}", self.format_message("ERROR", message));
    }

    pub fn debug(&self, message: &str) {
        debug!("{}", self.format_message("DEBUG", message));
    }

    pub fn trace(&self, message: &str) {
        trace!("{}", self.format_message("TRACE", message));
    }
}

/// Performance monitoring utilities
pub struct PerformanceMonitor {
    pub start_time: SystemTime,
    operation: String,
    metadata: HashMap<String, Value>,
}

impl PerformanceMonitor {
    pub fn new(operation: &str) -> Self {
        Self {
            start_time: SystemTime::now(),
            operation: operation.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    pub fn finish(self) -> u64 {
        let duration = SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut context = LogContext::new("performance", &self.operation)
            .with_duration_ms(duration);

        for (key, value) in self.metadata {
            context = context.with_metadata(&key, value);
        }

        context.info(&format!("Operation completed in {}ms", duration));
        duration
    }

    pub fn finish_with_result<T, E>(self, result: &Result<T, E>) -> u64 
    where 
        E: std::fmt::Display 
    {
        let duration = SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut context = LogContext::new("performance", &self.operation)
            .with_duration_ms(duration);

        for (key, value) in self.metadata {
            context = context.with_metadata(&key, value);
        }

        match result {
            Ok(_) => {
                context.info(&format!("Operation completed successfully in {}ms", duration));
            }
            Err(e) => {
                context = context.with_metadata("error", json!(e.to_string()));
                context.error(&format!("Operation failed after {}ms: {}", duration, e));
            }
        }

        duration
    }
}

/// Error logging utilities
pub struct ErrorLogger;

impl ErrorLogger {
    pub fn log_error(error: &crate::error::IndexerError, context: Option<LogContext>) {
        let severity = error.severity();
        let is_recoverable = error.is_recoverable();
        let retry_delay = error.retry_delay();

        let mut log_context = context.unwrap_or_else(|| LogContext::new("error", "unknown"));
        log_context = log_context
            .with_metadata("error_type", json!(format!("{:?}", error)))
            .with_metadata("severity", json!(format!("{:?}", severity)))
            .with_metadata("recoverable", json!(is_recoverable));

        if let Some(delay) = retry_delay {
            log_context = log_context.with_metadata("retry_delay_seconds", json!(delay));
        }

        let message = format!("Error occurred: {}", error);

        match severity {
            crate::error::ErrorSeverity::Critical => log_context.error(&message),
            crate::error::ErrorSeverity::High => log_context.error(&message),
            crate::error::ErrorSeverity::Medium => log_context.warn(&message),
            crate::error::ErrorSeverity::Low => log_context.info(&message),
        }
    }

    pub fn log_recovery_attempt(error: &crate::error::IndexerError, attempt: u32, max_attempts: u32) {
        let context = LogContext::new("recovery", "retry_attempt")
            .with_retry_count(attempt)
            .with_metadata("max_attempts", json!(max_attempts))
            .with_metadata("error_type", json!(format!("{:?}", error)));

        if attempt == max_attempts {
            context.error(&format!("Final retry attempt failed: {}", error));
        } else {
            context.warn(&format!("Retry attempt {} of {}: {}", attempt, max_attempts, error));
        }
    }

    pub fn log_recovery_success(operation: &str, attempts: u32, total_duration_ms: u64) {
        let context = LogContext::new("recovery", "success")
            .with_metadata("operation", json!(operation))
            .with_retry_count(attempts)
            .with_duration_ms(total_duration_ms);

        context.info(&format!("Operation recovered after {} attempts in {}ms", attempts, total_duration_ms));
    }
}

/// Application metrics and monitoring
pub struct MetricsLogger;

impl MetricsLogger {
    pub fn log_block_processed(block_number: u64, transfer_count: u32, processing_time_ms: u64) {
        let context = LogContext::new("metrics", "block_processed")
            .with_block_number(block_number)
            .with_metadata("transfer_count", json!(transfer_count))
            .with_duration_ms(processing_time_ms);

        context.info(&format!("Block {} processed with {} transfers", block_number, transfer_count));
    }

    pub fn log_net_flow_update(direction: &str, amount: &str, new_net_flow: &str) {
        let context = LogContext::new("metrics", "net_flow_update")
            .with_metadata("direction", json!(direction))
            .with_amount(amount)
            .with_metadata("new_net_flow", json!(new_net_flow));

        context.info(&format!("Net flow updated: {} {} POL, new net flow: {}", direction, amount, new_net_flow));
    }

    pub fn log_rpc_call(method: &str, duration_ms: u64, success: bool) {
        let context = LogContext::new("metrics", "rpc_call")
            .with_metadata("method", json!(method))
            .with_duration_ms(duration_ms)
            .with_metadata("success", json!(success));

        if success {
            context.debug(&format!("RPC call {} completed in {}ms", method, duration_ms));
        } else {
            context.warn(&format!("RPC call {} failed after {}ms", method, duration_ms));
        }
    }

    pub fn log_database_operation(operation: &str, duration_ms: u64, rows_affected: Option<usize>) {
        let mut context = LogContext::new("metrics", "database_operation")
            .with_metadata("operation", json!(operation))
            .with_duration_ms(duration_ms);

        if let Some(rows) = rows_affected {
            context = context.with_metadata("rows_affected", json!(rows));
        }

        context.debug(&format!("Database {} completed in {}ms", operation, duration_ms));
    }

    pub fn log_system_status(
        latest_block: u64,
        last_processed_block: u64,
        blocks_behind: u64,
        total_transactions: u64,
        current_net_flow: &str,
    ) {
        let context = LogContext::new("metrics", "system_status")
            .with_metadata("latest_block", json!(latest_block))
            .with_metadata("last_processed_block", json!(last_processed_block))
            .with_metadata("blocks_behind", json!(blocks_behind))
            .with_metadata("total_transactions", json!(total_transactions))
            .with_metadata("current_net_flow", json!(current_net_flow));

        if blocks_behind > 10 {
            context.warn(&format!("System is {} blocks behind (latest: {}, processed: {})", 
                blocks_behind, latest_block, last_processed_block));
        } else {
            context.info(&format!("System status: {} blocks behind, {} total transactions, net flow: {}", 
                blocks_behind, total_transactions, current_net_flow));
        }
    }
}

/// Initialize structured logging for the application
pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize env_logger with custom format
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            use std::io::Write;
            
            // Try to parse as JSON for structured logs
            if let Ok(json_value) = serde_json::from_str::<Value>(record.args().to_string().as_str()) {
                writeln!(buf, "{}", serde_json::to_string_pretty(&json_value)?)
            } else {
                // Fall back to standard format for non-structured logs
                writeln!(
                    buf,
                    "{} [{}] {}: {}",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    record.level(),
                    record.target(),
                    record.args()
                )
            }
        })
        .init();

    info!("Structured logging initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_log_context_creation() {
        let context = LogContext::new("test_component", "test_operation");
        assert_eq!(context.component, "test_component");
        assert_eq!(context.operation, "test_operation");
        assert!(context.metadata.is_empty());
    }

    #[test]
    fn test_log_context_with_metadata() {
        let context = LogContext::new("test", "test")
            .with_block_number(12345)
            .with_transaction_hash("0xabc123")
            .with_amount("100.5");

        assert_eq!(context.metadata.get("block_number"), Some(&json!(12345)));
        assert_eq!(context.metadata.get("transaction_hash"), Some(&json!("0xabc123")));
        assert_eq!(context.metadata.get("amount"), Some(&json!("100.5")));
    }

    #[test]
    fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new("test_operation")
            .with_metadata("test_key", json!("test_value"));

        assert_eq!(monitor.operation, "test_operation");
        assert_eq!(monitor.metadata.get("test_key"), Some(&json!("test_value")));
    }

    #[test]
    fn test_performance_monitor_with_result() {
        let monitor = PerformanceMonitor::new("test_operation");
        let result: Result<(), String> = Ok(());
        
        let duration = monitor.finish_with_result(&result);
        assert!(duration >= 0); // Duration should be non-negative
    }

    #[test]
    fn test_error_logging() {
        let error = crate::error::IndexerError::Config(
            crate::error::ConfigError::MissingEnvVar("TEST_VAR".to_string())
        );
        
        let context = LogContext::new("test", "error_test");
        
        // This should not panic
        ErrorLogger::log_error(&error, Some(context));
    }

    #[test]
    fn test_metrics_logging() {
        // These should not panic
        MetricsLogger::log_block_processed(12345, 5, 150);
        MetricsLogger::log_net_flow_update("inflow", "100.5", "1500.5");
        MetricsLogger::log_rpc_call("eth_getBlockByNumber", 250, true);
        MetricsLogger::log_database_operation("INSERT", 50, Some(1));
        MetricsLogger::log_system_status(12345, 12340, 5, 1000, "1500.5");
    }

    #[test]
    fn test_log_context_format_message() {
        let context = LogContext::new("test", "test")
            .with_metadata("key", json!("value"));
        
        let message = context.format_message("INFO", "test message");
        
        // Should be valid JSON
        let parsed: Value = serde_json::from_str(&message).expect("Should be valid JSON");
        assert_eq!(parsed["level"], "INFO");
        assert_eq!(parsed["component"], "test");
        assert_eq!(parsed["operation"], "test");
        assert_eq!(parsed["message"], "test message");
        assert_eq!(parsed["key"], "value");
    }
}