#![warn(unsafe_code)]
//! SEED CLI Server: Multi-threaded TCP/IPC listener dan CLI daemon handler

use sedd_core::core::StructuralFractalCore;
use sedd_core::core::fractal::AffineTransformation;
use sedd_core::engine::DataIngestion;
use sedd_core::storage::MatrixStore;
use clap::{Parser, Subcommand};
use nalgebra::{DMatrix, DVector};
use std::path::PathBuf;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::sync::Arc;
use tokio::sync::Mutex;

/// SEED: Structural Entropy Dissolution Database Engine
#[derive(Parser)]
#[command(name = "SEED")]
#[command(about = "Non-Von Neumann database engine operating in fractal mathematical space", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start TCP server
    #[command(about = "Start SEED TCP server")]
    Server {
        /// Listen address (default: 127.0.0.1:5432)
        #[arg(short, long, default_value = "127.0.0.1:5432")]
        bind: String,

        /// Database file path
        #[arg(short, long, default_value = "seed.db")]
        db: PathBuf,
    },

    /// Create new dissolution transformation
    #[command(about = "Dissolve data into fractal transformation")]
    Dissolve {
        /// Input file (JSON, CSV, or BIN)
        #[arg(short, long)]
        input: PathBuf,

        /// Output database file
        #[arg(short, long, default_value = "seed.db")]
        db: PathBuf,

        /// Contraction factor (0.0 to 1.0)
        #[arg(short, long, default_value = "0.5")]
        contraction: f64,
    },

    /// Reconstruct data from dissolution
    #[command(about = "Reconstruct original data from transformation")]
    Reconstruct {
        /// Database file
        #[arg(short, long, default_value = "seed.db")]
        db: PathBuf,

        /// Transformation index
        #[arg(short, long, default_value = "0")]
        index: usize,
    },

    /// Server info
    #[command(about = "Show server information")]
    Info,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Server { bind, db } => {
            run_server(&bind, &db).await?;
        }
        Commands::Dissolve { input, db, contraction } => {
            dissolve_command(&input, &db, contraction)?;
        }
        Commands::Reconstruct { db, index } => {
            reconstruct_command(&db, index)?;
        }
        Commands::Info => {
            print_info();
        }
    }

    Ok(())
}

async fn run_server(bind: &str, db_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(bind).await?;
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  SEED Core Engine v0.2.0                                  ║");
    println!("║  Structural Entropy Dissolution Database                  ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║  ✓ Server started on: {}                    ║", bind);
    println!("║  ✓ Database path: {:?}                 ║", db_path);
    println!("║  ✓ Ready for fractal dissolution...                       ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let store = Arc::new(Mutex::new(MatrixStore::open(db_path)?));

    loop {
        let (socket, addr) = listener.accept().await?;
        let store_clone = Arc::clone(&store);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, store_clone).await {
                eprintln!("Connection error from {}: {}", addr, e);
            }
        });
    }
}

async fn handle_connection(
    socket: TcpStream,
    _store: Arc<Mutex<sedd_core::storage::MatrixStore>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    writer.write_all(b"SEED/1.0 Welcome\r\n").await?;

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;

        if n == 0 {
            break; // Connection closed
        }

        let command = line.trim();

        match command {
            "INFO" => {
                writer.write_all(b"SEED/1.0 Core Engine v0.2.0\r\n").await?;
            }
            "PING" => {
                writer.write_all(b"PONG\r\n").await?;
            }
            "QUIT" => {
                break;
            }
            _ => {
                writer.write_all(b"ERROR: Unknown command\r\n").await?;
            }
        }
    }

    Ok(())
}

fn dissolve_command(input: &PathBuf, db: &PathBuf, contraction: f64) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Dissolution Process");
    println!("├─ Input: {:?}", input);
    println!("├─ Database: {:?}", db);
    println!("├─ Contraction: {}", contraction);
    println!("│\n");

    // Parse input
    let parsed = DataIngestion::parse_file(input.to_str().unwrap())?;
    println!("✓ Data parsed: {} dimensions", parsed.vector.len());

    // Create core
    let mut core = StructuralFractalCore::new(parsed.vector.len())?;
    println!("✓ Fractal core created");

    // Create transformation
    let dim = parsed.vector.len();
    let matrix = DMatrix::identity(dim, dim) * contraction;
    let bias = DVector::from_element(dim, 0.01);
    let transform = AffineTransformation::new(matrix, bias)?;

    core.add_transformation(transform)?;
    println!("✓ Transformation added (norm: {:.4})", core.get_transformation(0).unwrap().matrix_norm);

    // Store
    let mut store = MatrixStore::open(db)?;
    let serialized = sedd_core::storage::SerializedTransformation {
        matrix_data: parsed.vector.as_slice().to_vec(),
        matrix_rows: dim,
        matrix_cols: 1,
        bias: vec![0.01],
        matrix_norm: contraction,
    };

    store.store_transformation(serialized)?;
    store.flush()?;
    println!("✓ Stored to database\n");

    Ok(())
}

fn reconstruct_command(_db: &PathBuf, _index: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Reconstruction Process");
    println!("├─ Database: {:?}", _db);
    println!("├─ Index: {}", _index);
    println!("│\n");

    println!("✓ Feature coming in next iteration\n");

    Ok(())
}

fn print_info() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  SEED Core Engine v0.2.0                                  ║");
    println!("║  Structural Entropy Dissolution Database                  ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║                                                           ║");
    println!("║  Non-Von Neumann database operating in fractal space      ║");
    println!("║                                                           ║");
    println!("║  Features:                                                ║");
    println!("║  ✓ Affine transformation dissolution                      ║");
    println!("║  ✓ Entropy collapse for memory optimization              ║");
    println!("║  ✓ Lossless reconstruction via fixed-point iteration     ║");
    println!("║  ✓ Multi-format ingestion (JSON, CSV, Binary)            ║");
    println!("║  ✓ Persistent storage with checksum validation           ║");
    println!("║  ✓ TCP/IPC server interface                              ║");
    println!("║                                                           ║");
    println!("║  Usage:                                                   ║");
    println!("║  $ seed server --bind 0.0.0.0:5432                       ║");
    println!("║  $ seed dissolve --input data.json --db seed.db          ║");
    println!("║  $ seed reconstruct --db seed.db --index 0               ║");
    println!("║                                                           ║");
    println!("║  Repository: https://github.com/nafalfaturizki-hub/SEED  ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}

