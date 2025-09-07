#[cfg(test)]
mod tests {
    use crate::error::{IndexerError, RpcError, DatabaseError, ProcessingError, ConfigError, ErrorSeverity};
    use crate::retry::{RetryConfig, RetryManager, CircuitBreaker};
    use crate::logging::{LogContext, ErrorLogger};

    #[test]
    fn test_error_severity() {
        let critical_error = IndexerError::Config(ConfigError::MissingEnvVar("TEST".to_string()));
        assert_eq!(critical_error.severity(), ErrorSeverity::Critical);

        let high_error = IndexerError::Rpc(RpcError::Authentication);
        assert_eq!(high_error.severity(), ErrorSeverity::High);

        let medium_error = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        assert_eq!(medium_error.severity(), ErrorSeverity::Medium);

        let low_error = IndexerError::Rpc(RpcError::BlockNotFound { block_number: 123 });
        assert_eq!(low_error.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_error_recoverability() {
        // Recoverable errors
        let timeout_error = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        assert!(timeout_error.is_recoverable());
        assert_eq!(timeout_error.retry_delay(), Some(5));

        let rate_limit_error = IndexerError::Rpc(RpcError::RateLimit { seconds: 60 });
        assert!(rate_limit_error.is_recoverable());
        assert_eq!(rate_limit_error.retry_delay(), Some(60));

        let connection_error = IndexerError::Rpc(RpcError::Connection("Connection failed".to_string()));
        assert!(connection_error.is_recoverable());
        assert_eq!(connection_error.retry_delay(), Some(10));

        // Non-recoverable errors
        let config_error = IndexerError::Config(ConfigError::MissingEnvVar("TEST".to_string()));
        assert!(!config_error.is_recoverable());
        assert_eq!(config_error.retry_delay(), None);

        let auth_error = IndexerError::Rpc(RpcError::Authentication);
        assert!(!auth_error.is_recoverable());
        assert_eq!(auth_error.retry_delay(), None);
    }

    #[test]
    fn test_error_conversion_from_legacy() {
        // Test conversion from legacy RpcError
        let legacy_rpc_error = crate::blockchain::rpc_client::RpcError::Rpc("Test error".to_string());
        let new_rpc_error: RpcError = legacy_rpc_error.into();
        assert!(matches!(new_rpc_error, RpcError::InvalidResponse(_)));

        // Test conversion from legacy DbError
        let legacy_db_error = crate::database::DbError::NotFound;
        let new_db_error: DatabaseError = legacy_db_error.into();
        assert!(matches!(new_db_error, DatabaseError::NotFound(_)));
    }

    #[test]
    fn test_processing_error_types() {
        let block_parsing_error = ProcessingError::BlockParsing("Invalid block format".to_string());
        assert_eq!(format!("{}", block_parsing_error), "Block parsing failed: Invalid block format");

        let address_validation_error = ProcessingError::AddressValidation("Invalid address format".to_string());
        assert_eq!(format!("{}", address_validation_error), "Address validation failed: Invalid address format");

        let amount_parsing_error = ProcessingError::AmountParsing("Invalid amount format".to_string());
        assert_eq!(format!("{}", amount_parsing_error), "Amount parsing failed: Invalid amount format");

        let event_signature_error = ProcessingError::EventSignature {
            expected: "0xabc123".to_string(),
            got: "0xdef456".to_string(),
        };
        assert_eq!(format!("{}", event_signature_error), "Event signature mismatch: expected=0xabc123, got=0xdef456");
    }

    #[test]
    fn test_config_error_types() {
        let missing_env_error = ConfigError::MissingEnvVar("DATABASE_URL".to_string());
        assert_eq!(format!("{}", missing_env_error), "Missing required environment variable: DATABASE_URL");

        let invalid_value_error = ConfigError::InvalidValue {
            key: "PORT".to_string(),
            value: "invalid".to_string(),
        };
        assert_eq!(format!("{}", invalid_value_error), "Invalid configuration value for PORT: invalid");

        let invalid_url_error = ConfigError::InvalidUrl("not-a-url".to_string());
        assert_eq!(format!("{}", invalid_url_error), "Invalid URL format: not-a-url");
    }

    #[test]
    fn test_error_logging() {
        let error = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        let context = LogContext::new("test", "error_logging_test")
            .with_metadata("test_key", serde_json::json!("test_value"));

        // This should not panic and should log appropriately
        ErrorLogger::log_error(&error, Some(context));
        ErrorLogger::log_recovery_attempt(&error, 2, 5);
        ErrorLogger::log_recovery_success("test_operation", 3, 1500);
    }

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

    #[test]
    fn test_error_chain_display() {
        // Test that error chains display properly
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let system_error = crate::error::SystemError::FileSystem(io_error);
        let indexer_error = IndexerError::System(system_error);
        
        let error_string = format!("{}", indexer_error);
        assert!(error_string.contains("System error"));
        assert!(error_string.contains("File system error"));
    }

    #[tokio::test]
    async fn test_enhanced_retry_manager() {
        use crate::error_recovery::EnhancedRetryManager;
        
        let retry_config = RetryConfig {
            max_attempts: 2,
            initial_delay_seconds: 1,
            max_delay_seconds: 5,
            backoff_multiplier: 2.0,
            jitter: false,
        };
        
        let enhanced_manager = EnhancedRetryManager::new("test_operation", retry_config);
        
        // Test successful operation
        let result = enhanced_manager.execute_with_recovery(|| async {
            Ok::<i32, IndexerError>(42)
        }, "test_context").await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_enhanced_retry_manager_with_errors() {
        use crate::error_recovery::EnhancedRetryManager;
        
        let retry_config = RetryConfig {
            max_attempts: 2,
            initial_delay_seconds: 1,
            max_delay_seconds: 5,
            backoff_multiplier: 2.0,
            jitter: false,
        };
        
        let enhanced_manager = EnhancedRetryManager::new("test_operation", retry_config);
        
        // Test operation that always fails
        let result = enhanced_manager.execute_with_recovery(|| async {
            Err::<i32, IndexerError>(IndexerError::Rpc(RpcError::Timeout { seconds: 30 }))
        }, "test_context").await;
        
        assert!(result.is_err());
        
        // Check that error statistics were recorded
        let statistics = enhanced_manager.get_error_statistics().unwrap();
        assert!(!statistics.is_empty());
    }

    #[test]
    fn test_database_error_severity_classification() {
        use crate::error::DatabaseError;
        
        let connection_error = IndexerError::Database(DatabaseError::Connection(
            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(1), None)
        ));
        assert_eq!(connection_error.severity(), ErrorSeverity::Critical);
        
        let query_error = IndexerError::Database(DatabaseError::Query("SELECT failed".to_string()));
        assert_eq!(query_error.severity(), ErrorSeverity::Medium);
        
        let not_found_error = IndexerError::Database(DatabaseError::NotFound("Record not found".to_string()));
        assert_eq!(not_found_error.severity(), ErrorSeverity::Medium);
    }

    #[test]
    fn test_network_error_recoverability() {
        use crate::error::NetworkError;
        
        let timeout_error = IndexerError::Network(NetworkError::Timeout);
        assert!(timeout_error.is_recoverable());
        assert_eq!(timeout_error.retry_delay(), Some(5));
        
        let connection_refused = IndexerError::Network(NetworkError::ConnectionRefused);
        assert!(connection_refused.is_recoverable());
        assert_eq!(connection_refused.retry_delay(), Some(15));
        
        let unreachable = IndexerError::Network(NetworkError::Unreachable);
        assert_eq!(unreachable.severity(), ErrorSeverity::High);
    }

    #[test]
    fn test_processing_error_scenarios() {
        use crate::error::ProcessingError;
        
        let block_parsing_error = ProcessingError::BlockParsing("Invalid JSON".to_string());
        let indexer_error = IndexerError::Processing(block_parsing_error);
        assert_eq!(indexer_error.severity(), ErrorSeverity::Medium);
        assert!(!indexer_error.is_recoverable());
        
        let overflow_error = ProcessingError::Overflow("Amount too large".to_string());
        let indexer_error = IndexerError::Processing(overflow_error);
        assert_eq!(format!("{}", indexer_error), "Processing error: Calculation overflow: Amount too large");
    }

    #[test]
    fn test_validation_error_scenarios() {
        use crate::error::ValidationError;
        
        let invalid_address = ValidationError::InvalidAddress("0xinvalid".to_string());
        let indexer_error = IndexerError::Validation(invalid_address);
        assert_eq!(indexer_error.severity(), ErrorSeverity::Low);
        assert!(!indexer_error.is_recoverable());
        
        let out_of_range = ValidationError::OutOfRange("Block number too high".to_string());
        let indexer_error = IndexerError::Validation(out_of_range);
        assert_eq!(format!("{}", indexer_error), "Validation error: Value out of range: Block number too high");
    }

    #[test]
    fn test_system_error_scenarios() {
        use crate::error::SystemError;
        
        let out_of_memory = SystemError::OutOfMemory;
        let indexer_error = IndexerError::System(out_of_memory);
        assert_eq!(indexer_error.severity(), ErrorSeverity::Critical);
        assert!(!indexer_error.is_recoverable());
        
        let resource_exhausted = SystemError::ResourceExhausted("Too many connections".to_string());
        let indexer_error = IndexerError::System(resource_exhausted);
        assert!(indexer_error.is_recoverable());
        assert_eq!(indexer_error.retry_delay(), Some(30));
    }

    #[test]
    fn test_error_conversion_completeness() {
        // Test that all error conversions work properly
        let legacy_rpc_error = crate::blockchain::rpc_client::RpcError::Rpc("Test error".to_string());
        let converted: IndexerError = legacy_rpc_error.into();
        assert!(matches!(converted, IndexerError::Rpc(_)));
        
        let legacy_db_error = crate::database::DbError::Operation("Test error".to_string());
        let converted: IndexerError = legacy_db_error.into();
        assert!(matches!(converted, IndexerError::Database(_)));
    }

    #[test]
    fn test_error_metadata_preservation() {
        // Test that error metadata is preserved through conversions
        let rpc_error = RpcError::Method {
            code: -32601,
            message: "Method not found".to_string(),
        };
        let indexer_error = IndexerError::Rpc(rpc_error);
        
        let error_string = format!("{}", indexer_error);
        assert!(error_string.contains("-32601"));
        assert!(error_string.contains("Method not found"));
    }
}