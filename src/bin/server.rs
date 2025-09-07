use clap::Parser;
use polygon_pol_indexer::api::ApiServer;
use polygon_pol_indexer::database::Database;
use polygon_pol_indexer::config::AppConfig;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "polygon-pol-indexer-server")]
#[command(about = "HTTP API server for POL token net-flow data\nCreated by Agnivesh Kumar for Alfred Capital assignment")]
#[command(version = "0.1.0")]
struct Args {
    /// Database path
    #[arg(long, default_value = "./blockchain.db")]
    database: String,
    
    /// Server port
    #[arg(long, default_value = "8080")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Display server banner
    print_server_banner();
    
    // Initialize logging
    env_logger::init();

    let args = Args::parse();
    
    // Load configuration
    let config = AppConfig::load().unwrap_or_default();
    
    // Use CLI args or config values
    let db_path = if args.database != "./blockchain.db" {
        args.database
    } else {
        config.database.path
    };
    
    let port = if args.port != 8080 {
        args.port
    } else {
        config.api.port
    };

    // Initialize database
    let database = Database::new(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;
    let database = Arc::new(database);

    // Create and start API server
    let server = ApiServer::new(database, port);
    
    log::info!("Starting HTTP API server on port {}", args.port);
    
    if let Err(e) = server.start().await {
        log::error!("Server failed: {}", e);
        return Err(e.into());
    }

    Ok(())
}

fn print_server_banner() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              🌐 POL Token Indexer API Server 🌐             ║");
    println!("║                                                              ║");
    println!("║            HTTP API for blockchain data access              ║");
    println!("║                                                              ║");
    println!("║        Created by Agnivesh Kumar for Alfred Capital         ║");
    println!("║                        Assignment                            ║");
    println!("║                                                              ║");
    println!("║                Starting HTTP API server...                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}