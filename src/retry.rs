use std::time::Duration;
use tokio::time::sleep;
use crate::error::IndexerError;
use crate::logging::{LogContext, ErrorLogger, PerformanceMonitor};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries in seconds
    pub initial_delay_seconds: u64,
    /// Maximum delay between retries in seconds
    pub max_delay_seconds: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Whether to add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_seconds: 1,
            max_delay_seconds: 60,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a configuration for RPC operations
    pub fn for_rpc() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_seconds: 2,
            max_delay_seconds: 30,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a configuration for database operations
    pub fn for_database() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_seconds: 1,
            max_delay_seconds: 10,
            backoff_multiplier: 1.5,
            jitter: false,
        }
    }

    /// Create a configuration for network operations
    pub fn for_network() -> Self {
        Self {
            max_attempts: 7,
            initial_delay_seconds: 5,
            max_delay_seconds: 120,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a configuration for critical operations (fewer retries, faster failure)
    pub fn for_critical() -> Self {
        Self {
            max_attempts: 2,
            initial_delay_seconds: 1,
            max_delay_seconds: 5,
            backoff_multiplier: 2.0,
            jitter: false,
        }
    }
}

/// Retry mechanism with exponential backoff and jitter
pub struct RetryManager {
    config: RetryConfig,
    operation_name: String,
}

impl RetryManager {
    pub fn new(operation_name: &str, config: RetryConfig) -> Self {
        Self {
            config,
            operation_name: operation_name.to_string(),
        }
    }

    /// Execute an operation with retry logic
    pub async fn execute<T, F, Fut>(&self, operation: F) -> Result<T, IndexerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
    {
        let monitor = PerformanceMonitor::new(&format!("retry_{}", self.operation_name));
        let mut last_error = None;

        for attempt in 1..=self.config.max_attempts {
            let attempt_monitor = PerformanceMonitor::new(&format!("{}_attempt_{}", self.operation_name, attempt));
            
            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        ErrorLogger::log_recovery_success(
                            &self.operation_name,
                            attempt,
                            monitor.start_time.elapsed().unwrap_or_default().as_millis() as u64,
                        );
                    }
                    attempt_monitor.finish();
                    return Ok(result);
                }
                Err(error) => {
                    attempt_monitor.finish_with_result::<(), &IndexerError>(&Err(&error));
                    
                    // Check if error is recoverable
                    if !error.is_recoverable() {
                        let context = LogContext::new("retry", &self.operation_name)
                            .with_retry_count(attempt)
                            .with_metadata("reason", serde_json::json!("non_recoverable"));
                        context.error(&format!("Non-recoverable error, aborting retries: {}", error));
                        return Err(error);
                    }

                    // Check if we've reached max attempts
                    if attempt >= self.config.max_attempts {
                        ErrorLogger::log_recovery_attempt(&error, attempt, self.config.max_attempts);
                        last_error = Some(error);
                        break;
                    }

                    // Log retry attempt
                    ErrorLogger::log_recovery_attempt(&error, attempt, self.config.max_attempts);

                    // Calculate delay for next attempt
                    let delay = self.calculate_delay(attempt);
                    
                    let context = LogContext::new("retry", &self.operation_name)
                        .with_retry_count(attempt)
                        .with_metadata("delay_seconds", serde_json::json!(delay.as_secs()))
                        .with_metadata("max_attempts", serde_json::json!(self.config.max_attempts));
                    
                    context.info(&format!("Retrying in {} seconds (attempt {} of {})", 
                        delay.as_secs(), attempt, self.config.max_attempts));

                    sleep(delay).await;
                    last_error = Some(error);
                }
            }
        }

        // All retries exhausted
        let final_error = last_error.unwrap_or_else(|| {
            IndexerError::System(crate::error::SystemError::ResourceExhausted(
                "All retry attempts exhausted".to_string()
            ))
        });

        let context = LogContext::new("retry", &self.operation_name)
            .with_metadata("max_attempts", serde_json::json!(self.config.max_attempts));
        context.error(&format!("All {} retry attempts failed: {}", self.config.max_attempts, final_error));

        Err(final_error)
    }

    /// Execute an operation with retry logic and custom error handling
    pub async fn execute_with_handler<T, F, Fut, H>(
        &self,
        operation: F,
        error_handler: H,
    ) -> Result<T, IndexerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
        H: Fn(&IndexerError, u32) -> bool, // Returns true if should continue retrying
    {
        let monitor = PerformanceMonitor::new(&format!("retry_with_handler_{}", self.operation_name));
        let mut last_error = None;

        for attempt in 1..=self.config.max_attempts {
            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        ErrorLogger::log_recovery_success(
                            &self.operation_name,
                            attempt,
                            monitor.start_time.elapsed().unwrap_or_default().as_millis() as u64,
                        );
                    }
                    return Ok(result);
                }
                Err(error) => {
                    // Check custom error handler
                    if !error_handler(&error, attempt) {
                        let context = LogContext::new("retry", &self.operation_name)
                            .with_retry_count(attempt)
                            .with_metadata("reason", serde_json::json!("custom_handler_abort"));
                        context.error(&format!("Custom error handler aborted retries: {}", error));
                        return Err(error);
                    }

                    // Check if we've reached max attempts
                    if attempt >= self.config.max_attempts {
                        last_error = Some(error);
                        break;
                    }

                    // Calculate delay and wait
                    let delay = self.calculate_delay(attempt);
                    sleep(delay).await;
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            IndexerError::System(crate::error::SystemError::ResourceExhausted(
                "All retry attempts exhausted".to_string()
            ))
        }))
    }

    /// Calculate delay for the given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_delay_seconds as f64;
        let exponential_delay = base_delay * self.config.backoff_multiplier.powi(attempt as i32 - 1);
        
        // Cap at max delay
        let capped_delay = exponential_delay.min(self.config.max_delay_seconds as f64);
        
        // Add jitter if enabled
        let final_delay = if self.config.jitter {
            let jitter_factor = 0.1; // 10% jitter
            let jitter = capped_delay * jitter_factor * (rand::random::<f64>() - 0.5);
            (capped_delay + jitter).max(0.0)
        } else {
            capped_delay
        };

        Duration::from_secs_f64(final_delay)
    }
}

/// Convenience functions for common retry patterns
pub struct RetryUtils;

impl RetryUtils {
    /// Retry an RPC operation with standard configuration
    pub async fn retry_rpc<T, F, Fut>(operation_name: &str, operation: F) -> Result<T, IndexerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
    {
        let retry_manager = RetryManager::new(operation_name, RetryConfig::for_rpc());
        retry_manager.execute(operation).await
    }

    /// Retry a database operation with standard configuration
    pub async fn retry_database<T, F, Fut>(operation_name: &str, operation: F) -> Result<T, IndexerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
    {
        let retry_manager = RetryManager::new(operation_name, RetryConfig::for_database());
        retry_manager.execute(operation).await
    }

    /// Retry a network operation with standard configuration
    pub async fn retry_network<T, F, Fut>(operation_name: &str, operation: F) -> Result<T, IndexerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
    {
        let retry_manager = RetryManager::new(operation_name, RetryConfig::for_network());
        retry_manager.execute(operation).await
    }

    /// Retry with custom configuration
    pub async fn retry_with_config<T, F, Fut>(
        operation_name: &str,
        config: RetryConfig,
        operation: F,
    ) -> Result<T, IndexerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
    {
        let retry_manager = RetryManager::new(operation_name, config);
        retry_manager.execute(operation).await
    }
}

/// Circuit breaker pattern for preventing cascading failures
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout_seconds: u64,
    current_failures: std::sync::atomic::AtomicU32,
    last_failure_time: std::sync::Mutex<Option<std::time::Instant>>,
    state: std::sync::Mutex<CircuitBreakerState>,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitBreakerState {
    Closed,  // Normal operation
    Open,    // Failing, rejecting requests
    HalfOpen, // Testing if service recovered
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout_seconds: u64) -> Self {
        Self {
            failure_threshold,
            recovery_timeout_seconds,
            current_failures: std::sync::atomic::AtomicU32::new(0),
            last_failure_time: std::sync::Mutex::new(None),
            state: std::sync::Mutex::new(CircuitBreakerState::Closed),
        }
    }

    pub async fn execute<T, F, Fut>(&self, operation: F) -> Result<T, IndexerError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, IndexerError>>,
    {
        // Check current state
        let current_state = {
            let mut state = self.state.lock().unwrap();
            
            // Check if we should transition from Open to HalfOpen
            if *state == CircuitBreakerState::Open {
                if let Some(last_failure) = *self.last_failure_time.lock().unwrap() {
                    if last_failure.elapsed().as_secs() >= self.recovery_timeout_seconds {
                        *state = CircuitBreakerState::HalfOpen;
                        let context = LogContext::new("circuit_breaker", "state_transition")
                            .with_metadata("from", serde_json::json!("Open"))
                            .with_metadata("to", serde_json::json!("HalfOpen"));
                        context.info("Circuit breaker transitioning to HalfOpen state");
                    }
                }
            }
            
            state.clone()
        };

        match current_state {
            CircuitBreakerState::Open => {
                return Err(IndexerError::System(crate::error::SystemError::ResourceExhausted(
                    "Circuit breaker is open".to_string()
                )));
            }
            CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => {
                match operation().await {
                    Ok(result) => {
                        // Success - reset failure count and close circuit
                        self.current_failures.store(0, std::sync::atomic::Ordering::Relaxed);
                        *self.state.lock().unwrap() = CircuitBreakerState::Closed;
                        Ok(result)
                    }
                    Err(error) => {
                        // Failure - increment counter and potentially open circuit
                        let failures = self.current_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        *self.last_failure_time.lock().unwrap() = Some(std::time::Instant::now());

                        if failures >= self.failure_threshold {
                            *self.state.lock().unwrap() = CircuitBreakerState::Open;
                            let context = LogContext::new("circuit_breaker", "state_transition")
                                .with_metadata("from", serde_json::json!(format!("{:?}", current_state)))
                                .with_metadata("to", serde_json::json!("Open"))
                                .with_metadata("failure_count", serde_json::json!(failures));
                            context.error("Circuit breaker opened due to repeated failures");
                        }

                        Err(error)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay_seconds, 1);
        assert_eq!(config.max_delay_seconds, 60);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_presets() {
        let rpc_config = RetryConfig::for_rpc();
        assert_eq!(rpc_config.max_attempts, 5);
        assert_eq!(rpc_config.initial_delay_seconds, 2);

        let db_config = RetryConfig::for_database();
        assert_eq!(db_config.max_attempts, 3);
        assert!(!db_config.jitter);

        let critical_config = RetryConfig::for_critical();
        assert_eq!(critical_config.max_attempts, 2);
        assert_eq!(critical_config.max_delay_seconds, 5);
    }

    #[tokio::test]
    async fn test_retry_manager_success_on_first_attempt() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_seconds: 1,
            max_delay_seconds: 10,
            backoff_multiplier: 2.0,
            jitter: false,
        };

        let retry_manager = RetryManager::new("test_operation", config);
        
        let result = retry_manager.execute(|| async {
            Ok::<i32, IndexerError>(42)
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_manager_non_recoverable_error() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_seconds: 1,
            max_delay_seconds: 10,
            backoff_multiplier: 2.0,
            jitter: false,
        };

        let retry_manager = RetryManager::new("test_operation", config);
        
        let result = retry_manager.execute(|| async {
            Err::<i32, IndexerError>(IndexerError::Config(
                crate::error::ConfigError::MissingEnvVar("TEST".to_string())
            ))
        }).await;

        assert!(result.is_err());
        // Should fail immediately without retries for non-recoverable errors
    }

    #[tokio::test]
    async fn test_circuit_breaker_normal_operation() {
        let circuit_breaker = CircuitBreaker::new(3, 10);
        
        let result = circuit_breaker.execute(|| async {
            Ok::<i32, IndexerError>(42)
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let circuit_breaker = CircuitBreaker::new(2, 10);
        
        // First failure
        let result1 = circuit_breaker.execute(|| async {
            Err::<i32, IndexerError>(IndexerError::Network(
                crate::error::NetworkError::Timeout
            ))
        }).await;
        assert!(result1.is_err());

        // Second failure - should open circuit
        let result2 = circuit_breaker.execute(|| async {
            Err::<i32, IndexerError>(IndexerError::Network(
                crate::error::NetworkError::Timeout
            ))
        }).await;
        assert!(result2.is_err());

        // Third attempt - should be rejected due to open circuit
        let result3 = circuit_breaker.execute(|| async {
            Ok::<i32, IndexerError>(42)
        }).await;
        assert!(result3.is_err());
        assert!(result3.unwrap_err().to_string().contains("Circuit breaker is open"));
    }

    #[test]
    fn test_delay_calculation() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_delay_seconds: 2,
            max_delay_seconds: 30,
            backoff_multiplier: 2.0,
            jitter: false,
        };

        let retry_manager = RetryManager::new("test", config);

        let delay1 = retry_manager.calculate_delay(1);
        let delay2 = retry_manager.calculate_delay(2);
        let delay3 = retry_manager.calculate_delay(3);

        assert_eq!(delay1.as_secs(), 2);  // 2 * 2^0 = 2
        assert_eq!(delay2.as_secs(), 4);  // 2 * 2^1 = 4
        assert_eq!(delay3.as_secs(), 8);  // 2 * 2^2 = 8
    }

    #[test]
    fn test_delay_calculation_with_max_cap() {
        let config = RetryConfig {
            max_attempts: 10,
            initial_delay_seconds: 5,
            max_delay_seconds: 20,
            backoff_multiplier: 3.0,
            jitter: false,
        };

        let retry_manager = RetryManager::new("test", config);

        let delay5 = retry_manager.calculate_delay(5);
        // 5 * 3^4 = 5 * 81 = 405, but capped at 20
        assert_eq!(delay5.as_secs(), 20);
    }
}