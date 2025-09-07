use clap::Parser;
use polygon_pol_indexer::api::{CliHandler, Cli};
use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::config::AppConfig;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger for CLI (less verbose than the main indexer)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    
    // Display welcome banner
    print_banner();
    
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Load configuration and get database path
    let config = AppConfig::load().unwrap_or_default();
    let db_path = if cli.database != "./blockchain.db" {
        cli.database.clone()
    } else {
        config.database.path
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

fn print_banner() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              🔗 Polygon POL Token Indexer 🔗                ║");
    println!("║                                                              ║");
    println!("║           Real-time blockchain data analysis tool           ║");
    println!("║                                                              ║");
    println!("║        Created by Agnivesh Kumar for Alfred Capital         ║");
    println!("║                        Assignment                            ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}