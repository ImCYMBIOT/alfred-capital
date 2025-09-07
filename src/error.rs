use thiserror::Error;

/// Main error type for the Polygon POL Indexer application
#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcError),
    
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    
    #[error("Processing error: {0}")]
    Processing(#[from] ProcessingError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    
    #[error("System error: {0}")]
    System(#[from] SystemError),
}

/// RPC-related errors
#[derive(Error, Debug)]
pub enum RpcError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("RPC method error: code={code}, message={message}")]
    Method { code: i32, message: String },
    
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    
    #[error("Timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
    
    #[error("Rate limit exceeded, retry after {seconds} seconds")]
    RateLimit { seconds: u64 },
    
    #[error("Block not found: {block_number}")]
    BlockNotFound { block_number: u64 },
    
    #[error("Connection failed: {0}")]
    Connection(String),
    
    #[error("Authentication failed")]
    Authentication,
}

/// Database-related errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    Connection(#[from] rusqlite::Error),
    
    #[error("Transaction failed: {0}")]
    Transaction(String),
    
    #[error("Query failed: {0}")]
    Query(String),
    
    #[error("Data integrity violation: {0}")]
    Integrity(String),
    
    #[error("Lock acquisition failed: {0}")]
    Lock(String),
    
    #[error("Migration failed: {0}")]
    Migration(String),
    
    #[error("Backup failed: {0}")]
    Backup(String),
    
    #[error("Record not found: {0}")]
    NotFound(String),
    
    #[error("Constraint violation: {0}")]
    Constraint(String),
}

/// Block processing errors
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("Block parsing failed: {0}")]
    BlockParsing(String),
    
    #[error("Transaction parsing failed: {0}")]
    TransactionParsing(String),
    
    #[error("Log parsing failed: {0}")]
    LogParsing(String),
    
    #[error("Address validation failed: {0}")]
    AddressValidation(String),
    
    #[error("Amount parsing failed: {0}")]
    AmountParsing(String),
    
    #[error("Event signature mismatch: expected={expected}, got={got}")]
    EventSignature { expected: String, got: String },
    
    #[error("Insufficient log data: expected {expected} bytes, got {got}")]
    InsufficientData { expected: usize, got: usize },
    
    #[error("Invalid transfer direction")]
    InvalidDirection,
    
    #[error("Calculation overflow: {0}")]
    Overflow(String),
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    
    #[error("Invalid configuration value for {key}: {value}")]
    InvalidValue { key: String, value: String },
    
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),
    
    #[error("Configuration parsing failed: {0}")]
    Parsing(String),
    
    #[error("Invalid URL format: {0}")]
    InvalidUrl(String),
    
    #[error("Invalid port number: {0}")]
    InvalidPort(u16),
}

/// Network-related errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection timeout")]
    Timeout,
    
    #[error("DNS resolution failed: {0}")]
    DnsResolution(String),
    
    #[error("SSL/TLS error: {0}")]
    Tls(String),
    
    #[error("Network unreachable")]
    Unreachable,
    
    #[error("Connection refused")]
    ConnectionRefused,
    
    #[error("Too many redirects")]
    TooManyRedirects,
}

/// Validation errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid Ethereum address: {0}")]
    InvalidAddress(String),
    
    #[error("Invalid block number: {0}")]
    InvalidBlockNumber(String),
    
    #[error("Invalid transaction hash: {0}")]
    InvalidTransactionHash(String),
    
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),
    
    #[error("Value out of range: {0}")]
    OutOfRange(String),
}

/// System-level errors
#[derive(Error, Debug)]
pub enum SystemError {
    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),
    
    #[error("Memory allocation failed")]
    OutOfMemory,
    
    #[error("Thread panic: {0}")]
    ThreadPanic(String),
    
    #[error("Signal received: {0}")]
    Signal(String),
    
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, IndexerError>;

/// Error severity levels for logging and monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Critical errors that require immediate attention
    Critical,
    /// High priority errors that affect functionality
    High,
    /// Medium priority errors that may affect performance
    Medium,
    /// Low priority errors that are mostly informational
    Low,
}

impl IndexerError {
    /// Get the severity level of an error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            IndexerError::Database(DatabaseError::Connection(_)) => ErrorSeverity::Critical,
            IndexerError::Database(DatabaseError::Migration(_)) => ErrorSeverity::Critical,
            IndexerError::Config(_) => ErrorSeverity::Critical,
            IndexerError::System(SystemError::OutOfMemory) => ErrorSeverity::Critical,
            IndexerError::System(SystemError::PermissionDenied(_)) => ErrorSeverity::Critical,
            
            IndexerError::Rpc(RpcError::Connection(_)) => ErrorSeverity::High,
            IndexerError::Rpc(RpcError::Authentication) => ErrorSeverity::High,
            IndexerError::Database(DatabaseError::Transaction(_)) => ErrorSeverity::High,
            IndexerError::Database(DatabaseError::Integrity(_)) => ErrorSeverity::High,
            IndexerError::Network(NetworkError::Unreachable) => ErrorSeverity::High,
            
            IndexerError::Rpc(RpcError::Timeout { .. }) => ErrorSeverity::Medium,
            IndexerError::Rpc(RpcError::RateLimit { .. }) => ErrorSeverity::Medium,
            IndexerError::Processing(_) => ErrorSeverity::Medium,
            IndexerError::Database(DatabaseError::Query(_)) => ErrorSeverity::Medium,
            
            IndexerError::Validation(_) => ErrorSeverity::Low,
            IndexerError::Rpc(RpcError::BlockNotFound { .. }) => ErrorSeverity::Low,
            _ => ErrorSeverity::Medium,
        }
    }

    /// Check if the error is recoverable (can be retried)
    pub fn is_recoverable(&self) -> bool {
        match self {
            IndexerError::Rpc(RpcError::Timeout { .. }) => true,
            IndexerError::Rpc(RpcError::RateLimit { .. }) => true,
            IndexerError::Rpc(RpcError::Connection(_)) => true,
            IndexerError::Network(NetworkError::Timeout) => true,
            IndexerError::Network(NetworkError::ConnectionRefused) => true,
            IndexerError::Database(DatabaseError::Lock(_)) => true,
            IndexerError::System(SystemError::ResourceExhausted(_)) => true,
            
            // Non-recoverable errors
            IndexerError::Config(_) => false,
            IndexerError::Validation(_) => false,
            IndexerError::Rpc(RpcError::Authentication) => false,
            IndexerError::System(SystemError::PermissionDenied(_)) => false,
            
            _ => false,
        }
    }

    /// Get suggested retry delay in seconds for recoverable errors
    pub fn retry_delay(&self) -> Option<u64> {
        if !self.is_recoverable() {
            return None;
        }

        match self {
            IndexerError::Rpc(RpcError::RateLimit { seconds }) => Some(*seconds),
            IndexerError::Rpc(RpcError::Timeout { .. }) => Some(5),
            IndexerError::Rpc(RpcError::Connection(_)) => Some(10),
            IndexerError::Network(NetworkError::Timeout) => Some(5),
            IndexerError::Network(NetworkError::ConnectionRefused) => Some(15),
            IndexerError::Database(DatabaseError::Lock(_)) => Some(1),
            IndexerError::System(SystemError::ResourceExhausted(_)) => Some(30),
            _ => Some(5),
        }
    }
}

/// Convert from legacy error types for backward compatibility
impl From<crate::blockchain::rpc_client::RpcError> for RpcError {
    fn from(err: crate::blockchain::rpc_client::RpcError) -> Self {
        match err {
            crate::blockchain::rpc_client::RpcError::Http(e) => RpcError::Http(e),
            crate::blockchain::rpc_client::RpcError::Json(e) => RpcError::Json(e),
            crate::blockchain::rpc_client::RpcError::Rpc(msg) => {
                // Try to parse structured RPC errors
                if msg.contains("Code:") && msg.contains("Message:") {
                    // Parse "Code: -32601, Message: Method not found" format
                    if let Some(code_start) = msg.find("Code: ") {
                        if let Some(code_end) = msg[code_start + 6..].find(',') {
                            if let Ok(code) = msg[code_start + 6..code_start + 6 + code_end].parse::<i32>() {
                                if let Some(msg_start) = msg.find("Message: ") {
                                    let message = msg[msg_start + 9..].to_string();
                                    return RpcError::Method { code, message };
                                }
                            }
                        }
                    }
                }
                RpcError::InvalidResponse(msg)
            }
        }
    }
}

impl From<crate::database::DbError> for DatabaseError {
    fn from(err: crate::database::DbError) -> Self {
        match err {
            crate::database::DbError::Connection(e) => DatabaseError::Connection(e),
            crate::database::DbError::Operation(msg) => DatabaseError::Query(msg),
            crate::database::DbError::NotFound => DatabaseError::NotFound("Record not found".to_string()),
        }
    }
}

impl From<crate::blockchain::ProcessError> for ProcessingError {
    fn from(err: crate::blockchain::ProcessError) -> Self {
        // This will need to be implemented based on the actual ProcessError definition
        ProcessingError::BlockParsing(format!("{:?}", err))
    }
}

impl From<crate::blockchain::ProcessError> for IndexerError {
    fn from(err: crate::blockchain::ProcessError) -> Self {
        IndexerError::Processing(ProcessingError::from(err))
    }
}

impl From<crate::database::DbError> for IndexerError {
    fn from(err: crate::database::DbError) -> Self {
        IndexerError::Database(DatabaseError::from(err))
    }
}

impl From<crate::blockchain::rpc_client::RpcError> for IndexerError {
    fn from(err: crate::blockchain::rpc_client::RpcError) -> Self {
        IndexerError::Rpc(RpcError::from(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        let critical_error = IndexerError::Config(ConfigError::MissingEnvVar("TEST".to_string()));
        assert_eq!(critical_error.severity(), ErrorSeverity::Critical);

        let high_error = IndexerError::Rpc(RpcError::Authentication);
        assert_eq!(high_error.severity(), ErrorSeverity::High);

        let medium_error = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        assert_eq!(medium_error.severity(), ErrorSeverity::Medium);

        let low_error = IndexerError::Validation(ValidationError::InvalidAddress("0x123".to_string()));
        assert_eq!(low_error.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_error_recoverability() {
        let recoverable = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        assert!(recoverable.is_recoverable());

        let non_recoverable = IndexerError::Config(ConfigError::MissingEnvVar("TEST".to_string()));
        assert!(!non_recoverable.is_recoverable());
    }

    #[test]
    fn test_retry_delay() {
        let timeout_error = IndexerError::Rpc(RpcError::Timeout { seconds: 30 });
        assert_eq!(timeout_error.retry_delay(), Some(5));

        let rate_limit_error = IndexerError::Rpc(RpcError::RateLimit { seconds: 60 });
        assert_eq!(rate_limit_error.retry_delay(), Some(60));

        let non_recoverable = IndexerError::Config(ConfigError::MissingEnvVar("TEST".to_string()));
        assert_eq!(non_recoverable.retry_delay(), None);
    }

    #[test]
    fn test_error_display() {
        let error = IndexerError::Rpc(RpcError::Method {
            code: -32601,
            message: "Method not found".to_string(),
        });
        assert_eq!(format!("{}", error), "RPC error: RPC method error: code=-32601, message=Method not found");
    }

    #[test]
    fn test_error_chain() {
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let system_error = SystemError::FileSystem(io_error);
        let indexer_error = IndexerError::System(system_error);
        
        assert!(format!("{}", indexer_error).contains("File system error"));
    }
}