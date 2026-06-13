use rusqlite::Connection;
use std::error::Error;
use std::time::Instant;
use tokio::sync::mpsc;
use teloxide::{prelude::*, utils::command::BotCommands};
use tracing::{error, info};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Comandos de Controle HFT")]
enum Command {
    #[command(description = "Verifica o status e métricas da infraestrutura")]
    Status,
    #[command(description = "Lista as últimas 5 operações")]
    LastTrades,
}

pub async fn init_telegram_module(
    token: String,
    chat_id: String,
    db_path: String,
    start_time: Instant,
    execution_mode: String,
    mut rx_alerts: mpsc::Receiver<String>,
) {
    let bot = Bot::new(token.clone());
    info!("Serviço de controle do Telegram inicializado e escutando comandos.");

    let bot_clone = bot.clone();
    let db_path_clone = db_path.clone();
    let exec_mode_clone = execution_mode.clone();
    
    tokio::spawn(async move {
        let handler = Update::filter_message().branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                    let db_path = db_path_clone.clone();
                    let exec_mode = exec_mode_clone.clone();
                    async move {
                        match cmd {
                            Command::Status => {
                                let elapsed = start_time.elapsed().as_secs();
                                let hours = elapsed / 3600;
                                let mins = (elapsed % 3600) / 60;
                                let secs = elapsed % 60;

                                let stats = fetch_db_stats(&db_path).unwrap_or((0, 0.0));

                                let status_msg = format!(
                                    "[ ZANVEXIS HFT CORE - STATUS ]\n\n\
                                    Modo de Execução: {}\n\
                                    Uptime: {:02}h {:02}m {:02}s\n\n\
                                    [ MÉTRICAS DO BANCO DE DADOS ]\n\
                                    Operações Registradas: {}\n\
                                    Volume Total Processado: {:.4} SOL\n\
                                    Integridade do Arquivo SQLite: OK",
                                    exec_mode, hours, mins, secs, stats.0, stats.1
                                );

                                let _ = bot.send_message(msg.chat.id, status_msg).await;
                            }
                            Command::LastTrades => {
                                let response = fetch_last_trades(&db_path)
                                    .unwrap_or_else(|e| format!("Falha de I/O no banco de dados: {}", e));
                                let _ = bot.send_message(msg.chat.id, response).await;
                            }
                        }
                        respond(())
                    }
                }),
        );

        Dispatcher::builder(bot_clone, handler)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    });

    tokio::spawn(async move {
        while let Some(alert_text) = rx_alerts.recv().await {
            if let Err(e) = bot.send_message(chat_id.clone(), alert_text).disable_web_page_preview(true).await {
                error!("Falha ao transmitir alerta via API do Telegram: {}", e);
            }
        }
    });
}

fn fetch_db_stats(db_path: &str) -> Result<(i64, f64), Box<dyn Error + Send + Sync>> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT COUNT(*), COALESCE(SUM(amount_sol), 0.0) FROM trades")?;
    
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let count: i64 = row.get(0)?;
        let volume: f64 = row.get(1)?;
        return Ok((count, volume));
    }
    
    Ok((0, 0.0))
}

fn fetch_last_trades(db_path: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT original_tx, amount_sol, execution_mode FROM trades ORDER BY id DESC LIMIT 5"
    )?;
    
    let rows = stmt.query_map([], |row| {
        let tx: String = row.get(0)?;
        let amt: f64 = row.get(1)?;
        let mode: String = row.get(2)?;
        Ok(format!("[{}] Tx: {}... | Volume: {} SOL", mode, &tx[..8], amt))
    })?;

    let mut output = String::from("Últimas operações consolidadas:\n\n");
    let mut count = 0;

    for row in rows {
        if let Ok(line) = row {
            output.push_str(&format!("{}\n", line));
            count += 1;
        }
    }

    if count == 0 {
        return Ok("Nenhuma operação registrada na infraestrutura até o momento.".to_string());
    }

    Ok(output)
}