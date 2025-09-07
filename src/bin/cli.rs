use clap::Parser;
use polygon_pol_indexer::api::{CliHandler, Cli};
use polygon_pol_indexer::database::Database;
use std::sync::Arc;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger for CLI (less verbose than the main indexer)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Get database path from CLI args or environment variable
    let db_path = if cli.database != "./blockchain.db" {
        cli.database.clone()
    } else {
        env::var("DATABASE_PATH").unwrap_or_else(|_| cli.database.clone())
    };
    
    // Initialize database connection
    let database = match Database::new(&db_path) {
        Ok(db) => Arc::new(db),
        Err(e) => {
            eprintln!("Failed to connect to database at '{}': {}", db_path, e);
            eprintln!("Make sure the indexer has been run at least once to create the database.");
            std::process::exit(1);
        }
    };
    
    // Create CLI handler
    let cli_handler = CliHandler::new(database);
    
    // Execute the command
    if let Err(e) = cli_handler.execute_command(&cli.command).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    
    Ok(())
}