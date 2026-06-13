use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signer::Signer;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub mod classifier;
pub mod executor;
pub mod extractor;
pub mod ingestion;
pub mod models;
pub mod strategy;
pub mod telemetry;
pub mod telegram;
pub mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let start_time = Instant::now();

    let app_config = utils::load_config().expect("Falha ao carregar configuração.");
    info!("Executando bot. Modo: {:?}", app_config.execution_mode);

    let (tx_telemetry, rx_telemetry) = mpsc::channel::<models::TradeRecord>(5000);
    telemetry::start_telemetry_worker("storage/db/telemetry.db".to_string(), rx_telemetry).await;

    let (tx_alerts, rx_alerts) = mpsc::channel::<String>(1000);

    if let Some(tele_cfg) = app_config.telegram.clone() {
        if !tele_cfg.bot_token.is_empty() {
            telegram::init_telegram_module(
                tele_cfg.bot_token,
                tele_cfg.chat_id,
                "storage/db/telemetry.db".to_string(),
                start_time,
                format!("{:?}", app_config.execution_mode).to_uppercase(),
                rx_alerts
            ).await;
        }
    }

    let http_client = Client::builder()
        .timeout(std::time::Duration::from_millis(app_config.network.rpc_timeout_ms))
        .build()?;

    let rpc_client = RpcClient::new(app_config.rpc_url_http.clone());

    let bot_keypair = solana_sdk::signature::read_keypair_file(&app_config.wallet_path)
        .unwrap_or_else(|_| panic!("FALHA CRÍTICA: Arquivo de wallet não encontrado em {}", app_config.wallet_path));
    let bot_pubkey = bot_keypair.pubkey().to_string();
    tracing::info!("Identidade Persistente Carregada. Pubkey ativa: {}", bot_pubkey);

    let (tx_ingestion, mut rx_classifier) = mpsc::channel::<models::RawTransactionEvent>(10000);

    ingestion::start_stream(app_config.rpc_url_ws.clone(), tx_ingestion).await;

    while let Some(event) = rx_classifier.recv().await {
        if event.has_error { continue; }

        let intent = classifier::classify_pump_event(&event);

        let enriched = match intent {
            models::RawIntent::Buy { signature, slot } => {
                match extractor::fetch_trade_details(&http_client, &app_config.rpc_url_http, &signature).await {
                    Ok((wallet, mint, amount)) => Some(models::EnrichedTrade {
                        side: models::TradeSide::Buy, signature, mint, wallet, amount, slot
                    }),
                    Err(_) => continue,
                }
            }
            models::RawIntent::Sell { signature, slot } => {
                match extractor::fetch_trade_details(&http_client, &app_config.rpc_url_http, &signature).await {
                    Ok((wallet, mint, amount)) => Some(models::EnrichedTrade {
                        side: models::TradeSide::Sell, signature, mint, wallet, amount, slot
                    }),
                    Err(_) => continue,
                }
            }
            models::RawIntent::Irrelevant => None,
        };

        if let Some(trade_data) = enriched {
            if let Some(paper_trade) = strategy::evaluate_action(&trade_data, &app_config.trading) {
                
                let side_str = match paper_trade.side {
                    models::TradeSide::Buy => "COMPRA",
                    models::TradeSide::Sell => "VENDA",
                };
                
                tracing::info!("[ESTRATÉGIA APROVADA] Disparando ordem de {} para o Executor...", side_str);
                
                match executor::execute_trade(
                    &http_client, 
                    &rpc_client, 
                    &paper_trade, 
                    &app_config.trading, 
                    &bot_keypair, 
                    &app_config.execution_mode
                ).await {
                    Ok(exec_result) => {
                        let mode_string = format!("{:?}", exec_result.mode).to_uppercase();
                        
                        let record = models::TradeRecord {
                            execution_mode: mode_string.clone(),
                            original_tx: paper_trade.original_tx.clone(),
                            bot_tx: exec_result.signature.clone(),
                            mint: paper_trade.mint.clone(),
                            amount_sol: paper_trade.amount_sol_db,
                            slot: Some(paper_trade.slot),
                            price: None,
                            mc_origin: None,
                            mc_bot: None,
                            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        };

                        let _ = tx_telemetry.send(record).await;

                        let alert_msg = format!(
                            "[ ALERTA DE EXECUÇÃO : {} ]\n\nOperação: {}\nStatus: {:?}\nContrato: {}\nAssinatura: {}\nErro: {}",
                            mode_string, side_str, exec_result.status, paper_trade.mint, exec_result.signature,
                            exec_result.error_msg.unwrap_or_else(|| "Nenhum".to_string())
                        );
                        let _ = tx_alerts.send(alert_msg).await;
                    }
                    Err(e) => tracing::error!("Falha crítica no fluxo de execução: {}", e),
                }
            }
        }
    }

    Ok(())
}