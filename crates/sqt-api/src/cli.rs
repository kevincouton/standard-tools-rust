//! Command-line interface for the `standard-tools` binary.

use std::sync::Arc;

use clap::{Parser, Subcommand};
use sqt_audit::AuditVerifier;

use crate::services::{build_dispatcher, build_market_data_service};
use crate::state::AppState;
use sqt_audit::InMemoryStorage;

/// Standard-Tools CLI.
#[derive(Parser)]
#[command(name = "standard-tools")]
#[command(about = "Standard-Tools quant toolkit")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the HTTP/gRPC server.
    Server {
        /// HTTP port.
        #[arg(long, default_value = "8080")]
        http_port: u16,
        /// gRPC port.
        #[arg(long, default_value = "50051")]
        grpc_port: u16,
    },
    /// Audit trail commands.
    Audit {
        #[command(subcommand)]
        command: AuditCommands,
    },
}

#[derive(Subcommand)]
pub enum AuditCommands {
    /// Verify the audit chain.
    Verify,
    /// Replay the audit chain.
    Replay,
}

fn build_state() -> Arc<AppState<InMemoryStorage>> {
    let market_data = build_market_data_service();
    let dispatcher = Arc::new(build_dispatcher(market_data.clone()));
    let audit_storage = Arc::new(InMemoryStorage::new());
    let audit_writer = Arc::new(sqt_audit::AuditWriter::new(audit_storage.clone()));

    Arc::new(AppState {
        dispatcher,
        audit_writer,
        market_data,
    })
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Server {
            http_port,
            grpc_port,
        } => {
            crate::server::serve(build_state(), http_port, grpc_port).await?;
        }
        Commands::Audit { command } => match command {
            AuditCommands::Verify => {
                let state = build_state();
                let verifier = AuditVerifier::new(state.audit_writer.storage());
                let result = verifier.verify().await?;
                println!("{result:?}");
            }
            AuditCommands::Replay => {
                println!("replay not yet implemented from CLI");
            }
        },
    }
    Ok(())
}
