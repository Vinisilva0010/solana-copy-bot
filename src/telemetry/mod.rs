use crate::models::TradeRecord;
use rusqlite::{params, Connection};
use std::fs;
use tokio::sync::mpsc;
use tracing::{error, info};

pub async fn start_telemetry_worker(db_path: String, mut rx: mpsc::Receiver<TradeRecord>) {
    let init_path = db_path.clone();
    
    tokio::task::spawn_blocking(move || {
        if let Some(parent) = std::path::Path::new(&init_path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        match Connection::open(&init_path) {
            Ok(conn) => {
                let _ = conn.execute(
                    "CREATE TABLE IF NOT EXISTS trades (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        execution_mode TEXT NOT NULL,
                        original_tx TEXT NOT NULL,
                        bot_tx TEXT NOT NULL,
                        mint TEXT NOT NULL,
                        amount_sol REAL NOT NULL,
                        slot INTEGER,
                        price REAL,
                        mc_origin REAL,
                        mc_bot REAL,
                        timestamp INTEGER NOT NULL
                    )",
                    [],
                );
                info!("Banco de dados SQLite inicializado em {}", init_path);
            }
            Err(e) => error!("Falha ao inicializar o banco SQLite em {}: {}", init_path, e),
        }
    })
    .await
    .unwrap_or_else(|e| error!("Falha grave na task de inicialização do SQLite: {}", e));

    tokio::spawn(async move {
        while let Some(record) = rx.recv().await {
            let path = db_path.clone();
            
            tokio::task::spawn_blocking(move || {
                if let Ok(conn) = Connection::open(&path) {
                    let res = conn.execute(
                        "INSERT INTO trades (execution_mode, original_tx, bot_tx, mint, amount_sol, slot, price, mc_origin, mc_bot, timestamp)
                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
                            record.timestamp,
                        ],
                    );
                    if let Err(e) = res {
                        error!("Erro de I/O ao gravar telemetria no SQLite: {}", e);
                    }
                }
            });
        }
    });
}