use crate::models::TradeRecord;
use rusqlite::{params, Connection, Result};
use std::path::Path;
use tokio::sync::mpsc;
use tracing::{error, info};

pub fn init_db(db_path: &str) -> Result<Connection> {
    let path = Path::new(db_path);
    let conn = Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            execution_mode TEXT NOT NULL,
            original_tx TEXT NOT NULL,
            bot_tx TEXT NOT NULL,
            mint TEXT NOT NULL,
            amount_sol REAL NOT NULL,
            slot INTEGER NOT NULL,
            price REAL NOT NULL,
            mc_origin REAL NOT NULL,
            mc_bot REAL NOT NULL,
            timestamp INTEGER NOT NULL
        )",
        [],
    )?;

    info!("Banco de dados SQLite inicializado em {}", db_path);
    Ok(conn)
}

pub async fn start_telemetry_worker(db_path: String, mut rx_telemetry: mpsc::Receiver<TradeRecord>) {
    tokio::spawn(async move {
        // A conexão SQLite é mantida aberta dentro desta task isolada
        let conn = match init_db(&db_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Falha crítica ao iniciar banco de telemetria: {}", e);
                return;
            }
        };

        while let Some(record) = rx_telemetry.recv().await {
            let res = conn.execute(
                "INSERT INTO trades (
                    execution_mode, original_tx, bot_tx, mint, amount_sol, 
                    slot, price, mc_origin, mc_bot, timestamp
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    record.execution_mode,
                    record.original_tx,
                    record.bot_tx,
                    record.mint,
                    record.amount_sol,
                    record.slot,
                    record.price,
                    record.mc_origin,
                    record.mc_bot,
                    record.timestamp
                ],
            );

            if let Err(e) = res {
                error!("Falha ao gravar trade no banco de dados: {}", e);
            }
        }
    });
}