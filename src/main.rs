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
    info!("Executando bot. Modo: {}", app_config.execution_mode);

    let (tx_telemetry, rx_telemetry) = mpsc::channel::<models::TradeRecord>(5000);
    telemetry::start_telemetry_worker("storage/db/telemetry.db".to_string(), rx_telemetry).await;

    // Atualizado para refletir a nova estrutura aninhada do AppConfig
    if let Some(tele_cfg) = app_config.telegram.clone() {
        if !tele_cfg.bot_token.is_empty() {
            telegram::start_telemetry_service(
                tele_cfg.bot_token.clone(),
                "storage/db/telemetry.db".to_string(),
                start_time,
                app_config.execution_mode.clone()
            ).await;
        }
    }

    let http_client = Client::builder()
        .timeout(std::time::Duration::from_millis(app_config.network.rpc_timeout_ms))
        .build()?;

    let rpc_client = RpcClient::new(app_config.rpc_url_http.clone());

    // CARREGAMENTO DA WALLET PERSISTENTE
    let bot_keypair = solana_sdk::signature::read_keypair_file(&app_config.wallet_path)
        .unwrap_or_else(|_| panic!("FALHA CRÍTICA: Arquivo de wallet não encontrado em {}", app_config.wallet_path));
    let bot_pubkey = bot_keypair.pubkey().to_string();
    tracing::info!("Identidade Persistente Carregada. Pubkey ativa: {}", bot_pubkey);

    let (tx_ingestion, mut rx_classifier) = mpsc::channel::<models::RawTransactionEvent>(10000);

    ingestion::start_stream(app_config.rpc_url_ws.clone(), tx_ingestion).await;

    while let Some(event) = rx_classifier.recv().await {
        if event.has_error { continue; }

        let mut action = classifier::classify_pump_event(&event);

        match action {
            models::Action::Buy { ref mut mint, ref mut wallet, ref mut amount_sol, ref tx_origin, .. } => {
                match extractor::fetch_trade_details(&http_client, &app_config.rpc_url_http, tx_origin).await {
                    Ok((real_wallet, real_mint, real_amount)) => {
                        *wallet = real_wallet;
                        *mint = real_mint;
                        *amount_sol = real_amount;
                    }
                    Err(e) => {
                        tracing::debug!("RPC Helius - Falha na extração: {}", e);
                        continue;
                    }
                }
            }
            models::Action::Sell { ref mut mint, ref mut wallet, ref mut amount_tokens, ref tx_origin, .. } => {
                match extractor::fetch_trade_details(&http_client, &app_config.rpc_url_http, tx_origin).await {
                    Ok((real_wallet, real_mint, real_amount)) => {
                        *wallet = real_wallet;
                        *mint = real_mint;
                        *amount_tokens = real_amount;
                    }
                    Err(_) => continue,
                }
            }
            _ => { continue; }
        }

        if let Some(paper_trade) = strategy::evaluate_action(&action, &app_config.trading) {
            if paper_trade.side.as_str() == "BUY" {
                tracing::info!("📋 [ESTRATÉGIA APROVADA] Disparando payload para a rede...");
                
                match executor::build_transaction(&http_client, &paper_trade, &app_config.trading, &bot_pubkey).await {
                    Ok(transaction) => {
                        let tx_signature = transaction.signatures.get(0).map(|s| s.to_string()).unwrap_or_default();
                        
                        let _ = executor::execute_transaction(
                            &rpc_client, 
                            transaction, 
                            &bot_keypair, 
                            &app_config.execution_mode
                        ).await;

                        let record = models::TradeRecord {
                            execution_mode: app_config.execution_mode.clone(),
                            original_tx: paper_trade.original_tx.clone(),
                            bot_tx: tx_signature.clone(),
                            mint: paper_trade.mint.clone(),
                            amount_sol: paper_trade.execution_amount_sol,
                            slot: 0,
                            price: 0.0,
                            mc_origin: 0.0,
                            mc_bot: 0.0,
                            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        };

                        let _ = tx_telemetry.send(record).await;

                        let telegram_cfg = app_config.telegram.clone();
                        let mode_str = app_config.execution_mode.clone();
                        let client_clone = http_client.clone();
                        let amount = paper_trade.execution_amount_sol;
                        let target = paper_trade.mint.clone();

                        tokio::spawn(async move {
                            if let Some(tele_cfg) = telegram_cfg {
                                let alert_msg = format!(
                                    "[ ALERTA DE EXECUÇÃO : {} ]\n\n\
                                    Contrato Alvo: {}\n\
                                    Volume Processado: {:.4} SOL\n\
                                    Assinatura: {}",
                                    mode_str, target, amount, tx_signature
                                );
                                
                                let url = format!("https://api.telegram.org/bot{}/sendMessage", tele_cfg.bot_token);
                                let payload = serde_json::json!({
                                    "chat_id": tele_cfg.chat_id,
                                    "text": alert_msg,
                                    "disable_web_page_preview": true
                                });
                                
                                let _ = client_clone.post(&url).json(&payload).send().await;
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Falha na construção da transação: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}