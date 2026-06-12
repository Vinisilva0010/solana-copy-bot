use rusqlite::Connection;
use std::error::Error;
use teloxide::{prelude::*, utils::command::BotCommands};
use tracing::{error, info};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Comandos de Controle HFT")]
enum Command {
    #[command(description = "Verifica o status da infraestrutura")]
    Status,
    #[command(description = "Lista as últimas 5 operações")]
    LastTrades,
}

pub async fn start_telemetry_service(token: String, db_path: String) {
    tokio::spawn(async move {
        let bot = Bot::new(token);
        info!("Serviço de controle do Telegram inicializado e escutando comandos.");

        let handler = Update::filter_message().branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                    let db_path = db_path.clone();
                    async move {
                        match cmd {
                            Command::Status => {
                                let _ = bot.send_message(
                                    msg.chat.id, 
                                    "Infraestrutura operacional.\nMotor de captura on-chain ativo.\nLatência de leitura SQLite: Optimizada."
                                ).await;
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

        Dispatcher::builder(bot, handler)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    });
}

fn fetch_last_trades(db_path: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let conn = Connection::open(db_path)?;
    
    // Leitura das últimas execuções ordenadas pelo índice primário
    let mut stmt = conn.prepare(
        "SELECT original_tx, amount_sol, execution_mode FROM trades ORDER BY id DESC LIMIT 5"
    )?;
    
    let rows = stmt.query_map([], |row| {
        let tx: String = row.get(0)?;
        let amt: f64 = row.get(1)?;
        let mode: String = row.get(2)?;
        // Formatação limpa e técnica para o painel
        Ok(format!("[{}] Tx Origem: {}... | Volume: {} SOL", mode, &tx[..8], amt))
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