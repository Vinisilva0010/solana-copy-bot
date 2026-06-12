use tokio::sync::mpsc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod classifier;
pub mod executor;
pub mod ingestion;
pub mod models;
pub mod strategy;
pub mod telemetry;
pub mod telegram;
pub mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder().with_max_level(tracing::Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let app_config = utils::load_config().expect("Falha crítica ao carregar configuração.");
    tracing::info!("Executando bot. Modo: {}", app_config.execution_mode);

    let (tx_telemetry, rx_telemetry) = tokio::sync::mpsc::channel::<models::TradeRecord>(5000);
    telemetry::start_telemetry_worker("storage/db/telemetry.db".to_string(), rx_telemetry).await;

    // Inicializa a interface do Telegram caso o token exista no ambiente
    if let Some(telegram_token) = app_config.telegram_bot_token.clone() {
        if !telegram_token.is_empty() {
            telegram::start_telemetry_service(telegram_token, "storage/db/telemetry.db".to_string()).await;
        }
    }

    

    let http_client = Client::builder()
        .timeout(std::time::Duration::from_millis(app_config.network.rpc_timeout_ms))
        .build()?;

    let rpc_client = RpcClient::new(app_config.rpc_url_http.clone());

    let bot_keypair = Keypair::new();
    let bot_pubkey = bot_keypair.pubkey().to_string();
    info!("Keypair carregado. Pubkey ativa: {}", bot_pubkey);

    let (tx_ingestion, mut rx_classifier) = mpsc::channel::<models::RawTransactionEvent>(10000);

    ingestion::start_stream(app_config.rpc_url_ws.clone(), tx_ingestion).await;

    while let Some(event) = rx_classifier.recv().await {
        if event.has_error { continue; }

        let action = classifier::classify_pump_event(&event);

        if let Some(paper_trade) = strategy::evaluate_action(&action, &app_config.trading) {
            if paper_trade.side.as_str() == "BUY" {
                match executor::build_transaction(&http_client, &paper_trade, &app_config.trading, &bot_pubkey).await {
                    Ok(transaction) => {
                        let tx_signature = transaction.signatures.get(0).map(|s| s.to_string()).unwrap_or_default();
                        
                        let _ = executor::execute_transaction(
                            &rpc_client, 
                            transaction, 
                            &bot_keypair, 
                            &app_config.execution_mode
                        ).await;

                        // Gravação não bloqueante no banco SQLite
                        let record = models::TradeRecord {
                            execution_mode: app_config.execution_mode.clone(),
                            original_tx: paper_trade.original_tx.clone(),
                            bot_tx: tx_signature,
                            mint: paper_trade.mint.clone(),
                            amount_sol: paper_trade.execution_amount_sol,
                            slot: 0, // Mockado até implementarmos get_slot no executor
                            price: 0.0, // Preços serão populados via Bitquery no futuro
                            mc_origin: 0.0,
                            mc_bot: 0.0,
                            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        };

                        let _ = tx_telemetry.send(record).await;
                    }
                    Err(e) => {
                        error!("Falha ao construir transação: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}