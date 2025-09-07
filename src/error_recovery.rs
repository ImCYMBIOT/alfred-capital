use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::time::sleep;
use crate::error::{IndexerError, ErrorSeverity};
use crate::logging::{LogContext, ErrorLogger, PerformanceMonitor};
use crate::retry::{RetryConfig, RetryManager};

/// Advanced error recovery strategies for different types of failures
pub struct ErrorRecoveryManager {
    /// Track error patterns to identify systemic issues
    error_patterns: std::sync::Mutex<HashMap<String, ErrorPattern>>,
    /// Configuration for different recovery strategies
    recovery_configs: HashMap<String, RecoveryStrategy>,
}

#[derive(Debug, Clone)]
struct ErrorPattern {
    count: u32,
    first_occurrence: Instant,
    last_occurrence: Instant,
    error_type: String,
}

#[derive(Debug, Clone)]
pub struct RecoveryStrategy {
    pub max_attempts: u32,
    pub backoff_multiplier: f64,
    pub max_delay_seconds: u64,
    pub circuit_breaker_threshold: u32,
    pub recovery_actions: Vec<RecoveryAction>,
}

#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Wait for a specified duration before retrying
    Wait(Duration),
    /// Switch to backup RPC endpoint
    SwitchRpcEndpoint,
    /// Restart database connection
    RestartDatabaseConnection,
    /// Clear internal caches
    ClearCaches,
    /// Reduce processing load temporarily
    ReduceLoad,
    /// Send alert to monitoring system
    SendAlert(String),
    /// Perform health check
    HealthCheck,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_multiplier: 2.0,
            max_delay_seconds: 60,
            circuit_breaker_threshold: 5,
            recovery_actions: vec![
                RecoveryAction::Wait(Duration::from_secs(5)),
                RecoveryAction::HealthCheck,
            ],
        }
    }
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        let mut recovery_configs = HashMap::new();
        
        // RPC-specific recovery strategy
        recovery_configs.insert("rpc".to_string(), RecoveryStrategy {
            max_attempts: 5,
            backoff_multiplier: 2.0,
            max_delay_seconds: 120,
            circuit_breaker_threshold: 10,
            recovery_actions: vec![
                RecoveryAction::Wait(Duration::from_secs(5)),
                RecoveryAction::SwitchRpcEndpoint,
                RecoveryAction::HealthCheck,
                RecoveryAction::SendAlert("RPC connection issues detected".to_string()),
            ],
        });
        
        // Database-specific recovery strategy
        recovery_configs.insert("database".to_string(), RecoveryStrategy {
            max_attempts: 3,
            backoff_multiplier: 1.5,
            max_delay_seconds: 30,
            circuit_breaker_threshold: 5,
            recovery_actions: vec![
                RecoveryAction::Wait(Duration::from_secs(2)),
                RecoveryAction::RestartDatabaseConnection,
                RecoveryAction::ClearCaches,
                RecoveryAction::SendAlert("Database connection issues detected".to_string()),
            ],
        });
        
        // Network-specific recovery strategy
        recovery_configs.insert("network".to_string(), RecoveryStrategy {
            max_attempts: 7,
            backoff_multiplier: 2.0,
            max_delay_seconds: 300,
            circuit_breaker_threshold: 15,
            recovery_actions: vec![
                RecoveryAction::Wait(Duration::from_secs(10)),
                RecoveryAction::HealthCheck,
                RecoveryAction::ReduceLoad,
                RecoveryAction::SendAlert("Network connectivity issues detected".to_string()),
            ],
        });
        
        // Processing-specific recovery strategy
        recovery_configs.insert("processing".to_string(), RecoveryStrategy {
            max_attempts: 3,
            backoff_multiplier: 1.5,
            max_delay_seconds: 60,
            circuit_breaker_threshold: 8,
            recovery_actions: vec![
                RecoveryAction::Wait(Duration::from_secs(3)),
                RecoveryAction::ClearCaches,
                RecoveryAction::ReduceLoad,
            ],
        });
        
        Self {
            error_patterns: std::sync::Mutex::new(HashMap::new()),
            recovery_configs,
        }
    }
    
    /// Record an error occurrence and analyze patterns
    pub fn record_error(&self, error: &IndexerError, context: &str) {
        let error_type = format!("{:?}", error);
        let now = Instant::now();
        
        if let Ok(mut patterns) = self.error_patterns.lock() {
            let pattern = patterns.entry(error_type.clone()).or_insert(ErrorPattern {
                count: 0,
                first_occurrence: now,
                last_occurrence: now,
                error_type: error_type.clone(),
            });
            
            pattern.count += 1;
            pattern.last_occurrence = now;
            
            // Log pattern analysis
            let context = LogContext::new("error_recovery", "pattern_analysis")
                .with_metadata("error_type", serde_json::json!(error_type))
                .with_metadata("count", serde_json::json!(pattern.count))
                .with_metadata("context", serde_json::json!(context))
                .with_metadata("duration_since_first", serde_json::json!(
                    now.duration_since(pattern.first_occurrence).as_secs()
                ));
            
            if pattern.count >= 5 {
                context.warn(&format!("Error pattern detected: {} occurrences of {}", 
                    pattern.count, error_type));
            } else {
                context.debug(&format!("Error recorded: {} (count: {})", error_type, pattern.count));
            }
        }
    }
    
    /// Get recovery strategy for a specific error type
    pub fn get_recovery_strategy(&self, error: &IndexerError) -> RecoveryStrategy {
        let strategy_key = match error {
            IndexerError::Rpc(_) => "rpc",
            IndexerError::Database(_) => "database",
            IndexerError::Network(_) => "network",
            IndexerError::Processing(_) => "processing",
            _ => "default",
        };
        
        self.recovery_configs.get(strategy_key)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Execute recovery actions for an error
    pub async fn execute_recovery(&self, error: &IndexerError, context: &str) -> Result<(), IndexerError> {
        let strategy = self.get_recovery_strategy(error);
        
        let log_context = LogContext::new("error_recovery", "execute_recovery")
            .with_metadata("error_type", serde_json::json!(format!("{:?}", error)))
            .with_metadata("context", serde_json::json!(context))
            .with_metadata("strategy", serde_json::json!(format!("{:?}", strategy)));
        
        log_context.info("Executing error recovery strategy");
        
        for action in &strategy.recovery_actions {
            match self.execute_recovery_action(action, error).await {
                Ok(_) => {
                    let action_context = LogContext::new("error_recovery", "action_success")
                        .with_metadata("action", serde_json::json!(format!("{:?}", action)));
                    action_context.debug("Recovery action completed successfully");
                }
                Err(e) => {
                    let action_context = LogContext::new("error_recovery", "action_failure")
                        .with_metadata("action", serde_json::json!(format!("{:?}", action)))
                        .with_metadata("error", serde_json::json!(e.to_string()));
                    action_context.warn("Recovery action failed");
                }
            }
        }
        
        Ok(())
    }
    
    /// Execute a specific recovery action
    async fn execute_recovery_action(&self, action: &RecoveryAction, _error: &IndexerError) -> Result<(), IndexerError> {
        match action {
            RecoveryAction::Wait(duration) => {
                let context = LogContext::new("error_recovery", "wait")
                    .with_metadata("duration_seconds", serde_json::json!(duration.as_secs()));
                context.debug(&format!("Waiting {} seconds for recovery", duration.as_secs()));
                sleep(*duration).await;
                Ok(())
            }
            RecoveryAction::SwitchRpcEndpoint => {
                let context = LogContext::new("error_recovery", "switch_rpc_endpoint");
                context.info("Attempting to switch RPC endpoint");
                // In a real implementation, this would switch to a backup RPC endpoint
                // For now, we'll just log the action
                Ok(())
            }
            RecoveryAction::RestartDatabaseConnection => {
                let context = LogContext::new("error_recovery", "restart_database_connection");
                context.info("Attempting to restart database connection");
                // In a real implementation, this would restart the database connection
                // For now, we'll just log the action
                Ok(())
            }
            RecoveryAction::ClearCaches => {
                let context = LogContext::new("error_recovery", "clear_caches");
                context.info("Clearing internal caches");
                // In a real implementation, this would clear various caches
                // For now, we'll just log the action
                Ok(())
            }
            RecoveryAction::ReduceLoad => {
                let context = LogContext::new("error_recovery", "reduce_load");
                context.info("Reducing processing load temporarily");
                // In a real implementation, this would reduce the processing load
                // For now, we'll just log the action
                Ok(())
            }
            RecoveryAction::SendAlert(message) => {
                let context = LogContext::new("error_recovery", "send_alert")
                    .with_metadata("alert_message", serde_json::json!(message));
                context.warn(&format!("ALERT: {}", message));
                // In a real implementation, this would send alerts to monitoring systems
                Ok(())
            }
            RecoveryAction::HealthCheck => {
                let context = LogContext::new("error_recovery", "health_check");
                context.info("Performing system health check");
                // In a real implementation, this would perform comprehensive health checks
                // For now, we'll just log the action
                Ok(())
            }
        }
    }
    
    /// Get error pattern statistics
    pub fn get_error_statistics(&self) -> Result<Vec<ErrorStatistic>, IndexerError> {
        let patterns = self.error_patterns.lock().map_err(|_| {
            IndexerError::System(crate::error::SystemError::ResourceExhausted(
                "Error patterns lock poisoned".to_string()
            ))
        })?;
        
        let mut statistics = Vec::new();
        for (error_type, pattern) in patterns.iter() {
            statistics.push(ErrorStatistic {
                error_type: error_type.clone(),
                count: pattern.count,
                first_occurrence: pattern.first_occurrence,
                last_occurrence: pattern.last_occurrence,
                frequency: pattern.count as f64 / 
                    pattern.last_occurrence.duration_since(pattern.first_occurrence).as_secs_f64().max(1.0),
            });
        }
        
        // Sort by count descending
        statistics.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(statistics)
    }
    
    /// Check if an error type is showing concerning patterns
    pub fn is_error_pattern_concerning(&self, error: &IndexerError) -> bool {
        let error_type = format!("{:?}", error);
        
        if let Ok(patterns) = self.error_patterns.lock() {
            if let Some(pattern) = patterns.get(&error_type) {
                let duration_since_first = pattern.last_occurrence.duration_since(pattern.first_occurrence);
                let frequency = pattern.count as f64 / duration_since_first.as_secs_f64().max(1.0);
                
                // Consider it concerning if:
                // 1. More than 10 occurrences in the last hour
                // 2. More than 5 occurrences per minute on average
                // 3. More than 50 total occurrences
                return pattern.count > 50 || 
                       (duration_since_first.as_secs() < 3600 && pattern.count > 10) ||
                       frequency > 5.0 / 60.0;
            }
        }
        
        false
    }
}

#[derive(Debug, Clone)]
pub struct ErrorStatistic {
    pub error_type: String,
    pub count: u32,
    pub first_occurrence: Instant,
    pub last_occurrence: Instant,
    pub frequency: f64, // errors per second
}

/// Enhanced retry manager that integrates with error recovery
pub struct EnhancedRetryManager {
    base_manager: RetryManager,
    recovery_manager: ErrorRecoveryManager,
}

impl EnhancedRetryManager {
    pub fn new(operation_name: &str, config: RetryConfig) -> Self {
        Self {
            base_manager: RetryManager::new(operation_name, config),
            recovery_manager: ErrorRecoveryManager::new(),
        }
    }
    
    /// Execute operation with enhanced error recovery
    pub async fn execute_with_recovery<T, F, Fut>(
        &self,
        operation: F,
        context: &str,
    ) -> Result<T, IndexerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
    {
        let monitor = PerformanceMonitor::new(&format!("enhanced_retry_{}", context));
        
        let result = self.base_manager.execute(|| async {
            match operation().await {
                Ok(result) => Ok(result),
                Err(error) => {
                    // Record the error for pattern analysis
                    self.recovery_manager.record_error(&error, context);
                    
                    // Check if this error pattern is concerning
                    if self.recovery_manager.is_error_pattern_concerning(&error) {
                        let alert_context = LogContext::new("error_recovery", "concerning_pattern")
                            .with_metadata("error_type", serde_json::json!(format!("{:?}", error)))
                            .with_metadata("context", serde_json::json!(context));
                        alert_context.error("Concerning error pattern detected, executing recovery");
                        
                        // Execute recovery actions
                        if let Err(recovery_error) = self.recovery_manager.execute_recovery(&error, context).await {
                            let recovery_context = LogContext::new("error_recovery", "recovery_failed")
                                .with_metadata("original_error", serde_json::json!(error.to_string()))
                                .with_metadata("recovery_error", serde_json::json!(recovery_error.to_string()));
                            recovery_context.error("Error recovery failed");
                        }
                    }
                    
                    Err(error)
                }
            }
        }).await;
        
        monitor.finish_with_result(&result);
        result
    }
    
    /// Get error statistics from the recovery manager
    pub fn get_error_statistics(&self) -> Result<Vec<ErrorStatistic>, IndexerError> {
        self.recovery_manager.get_error_statistics()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{IndexerError, RpcError};

    #[test]
    fn test_error_recovery_manager_creation() {
        let manager = ErrorRecoveryManager::new();
        assert!(manager.recovery_configs.contains_key("rpc"));
        assert!(manager.recovery_configs.contains_key("database"));
        assert!(manager.recovery_configs.contains_key("network"));
        assert!(manager.recovery_configs.contains_key("processing"));
    }

    #[test]
    fn test_recovery_strategy_selection() {
        let manager = ErrorRecoveryManager::new();
        
        let rpc_error = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        let strategy = manager.get_recovery_strategy(&rpc_error);
        assert_eq!(strategy.max_attempts, 5);
        assert_eq!(strategy.circuit_breaker_threshold, 10);
        
        let db_error = IndexerError::Database(crate::error::DatabaseError::Connection(
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), None)
        ));
        let strategy = manager.get_recovery_strategy(&db_error);
        assert_eq!(strategy.max_attempts, 3);
        assert_eq!(strategy.circuit_breaker_threshold, 5);
    }

    #[test]
    fn test_error_pattern_recording() {
        let manager = ErrorRecoveryManager::new();
        let error = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        
        // Record the same error multiple times
        for _ in 0..3 {
            manager.record_error(&error, "test_context");
        }
        
        let statistics = manager.get_error_statistics().unwrap();
        assert!(!statistics.is_empty());
        
        let rpc_stat = statistics.iter()
            .find(|s| s.error_type.contains("Rpc"))
            .expect("Should find RPC error statistic");
        assert_eq!(rpc_stat.count, 3);
    }

    #[tokio::test]
    async fn test_recovery_action_execution() {
        let manager = ErrorRecoveryManager::new();
        let error = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        
        // This should not panic and should complete successfully
        let result = manager.execute_recovery(&error, "test_context").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_concerning_pattern_detection() {
        let manager = ErrorRecoveryManager::new();
        let error = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        
        // Initially should not be concerning
        assert!(!manager.is_error_pattern_concerning(&error));
        
        // Record many errors to make it concerning
        for _ in 0..60 {
            manager.record_error(&error, "test_context");
        }
        
        // Now it should be concerning
        assert!(manager.is_error_pattern_concerning(&error));
    }

    #[test]
    fn test_error_statistic_sorting() {
        let manager = ErrorRecoveryManager::new();
        
        let error1 = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        let error2 = IndexerError::Database(crate::error::DatabaseError::Connection(
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), None)
        ));
        
        // Record different numbers of each error
        for _ in 0..5 {
            manager.record_error(&error1, "test_context");
        }
        for _ in 0..3 {
            manager.record_error(&error2, "test_context");
        }
        
        let statistics = manager.get_error_statistics().unwrap();
        assert!(statistics.len() >= 2);
        
        // Should be sorted by count descending
        if statistics.len() >= 2 {
            assert!(statistics[0].count >= statistics[1].count);
        }
    }
}