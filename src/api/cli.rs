use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("CLI operation failed: {0}")]
    Operation(String),
}

pub struct CliHandler {
    // TODO: Implement CLI interface in future tasks
}