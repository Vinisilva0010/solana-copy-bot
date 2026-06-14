use crate::models::{TradeRecord, SystemHealth};
use rusqlite::{params, Connection};
use std::fs;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{error, info};

pub async fn start_telemetry_worker(
    db_path: String, 
    mut rx: mpsc::Receiver<TradeRecord>,
    health: Arc<RwLock<SystemHealth>>
) {
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
                        original_tx TEXT NOT NULL,
                        original_mint TEXT NOT NULL,
                        original_amount_sol REAL NOT NULL,
                        original_slot INTEGER NOT NULL,
                        bot_side TEXT NOT NULL,
                        execution_mode TEXT NOT NULL,
                        bot_tx TEXT,
                        bot_status TEXT NOT NULL,
                        units_consumed INTEGER NOT NULL,
                        timestamp INTEGER NOT NULL
                    )",
                    [],
                );
                info!("Banco de dados SQLite inicializado com schema profissional.");
            }
            Err(e) => error!("Falha ao inicializar o banco SQLite: {}", e),
        }
    })
    .await
    .unwrap_or_else(|e| error!("Falha grave na task de inicialização: {}", e));

    tokio::spawn(async move {
        while let Some(record) = rx.recv().await {
            let path = db_path.clone();
            let health_clone = health.clone();
            
            tokio::task::spawn_blocking(move || {
                if let Ok(conn) = Connection::open(&path) {
                    let res = conn.execute(
                        "INSERT INTO trades (
                            original_tx, original_mint, original_amount_sol, original_slot, 
                            bot_side, execution_mode, bot_tx, bot_status, units_consumed, timestamp
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        params![
                            record.original_tx, record.original_mint, record.original_amount_sol, record.original_slot,
                            record.bot_side, record.execution_mode, record.bot_tx, record.bot_status, 
                            record.units_consumed, record.timestamp,
                        ],
                    );
                    
                    if let Err(e) = res {
                        error!("Erro de I/O ao gravar telemetria: {}", e);
                    } else {
                        if let Ok(mut h) = health_clone.write() {
                            h.last_db_write = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                        }
                    }
                }
            });
        }
    });
}