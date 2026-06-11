use tokio::sync::mpsc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use reqwest::Client;

pub mod classifier;
pub mod executor;
pub mod ingestion;
pub mod models;
pub mod strategy;
pub mod telemetry;
pub mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let app_config = utils::load_config().expect("Falha ao carregar configuração.");
    info!("Executando bot. Modo: {}", app_config.execution_mode);

    // Cria cliente HTTP com pool de conexões (alta performance)
    let http_client = Client::builder()
        .timeout(std::time::Duration::from_millis(app_config.network.rpc_timeout_ms))
        .build()?;

    let (tx_ingestion, mut rx_classifier) = mpsc::channel::<models::RawTransactionEvent>(10000);

    ingestion::start_stream(app_config.rpc_url_ws.clone(), tx_ingestion).await;

    // Chave pública mockada da sua wallet (no Passo 6 puxaremos do keypair real)
    let bot_pubkey = "SuaWalletPublicaBot111111111111111111111111";

    while let Some(event) = rx_classifier.recv().await {
        if event.has_error { continue; }

        let action = classifier::classify_pump_event(&event);

        if let Some(paper_trade) = strategy::evaluate_action(&action, &app_config.trading) {
            match paper_trade.side.as_str() {
                "BUY" => {
                    info!("📋 [PAPER TRADE] Aprovado. Solicitando payload ao PumpPortal...");
                    
                    // Chama o executor para criar a transação
                    match executor::build_transaction(&http_client, &paper_trade, &app_config.trading, bot_pubkey).await {
                        Ok(_) => {
                            // Transação está pronta em memória. No próximo passo, simularemos.
                        }
                        Err(e) => {
                            error!("Falha ao construir transação: {}", e);
                        }
                    }
                }
                "SELL" => {
                    info!("📋 [PAPER TRADE] VENDA APROVADA | Mint: {}", paper_trade.mint);
                }
                _ => {}
            }
        }
    }

    Ok(())
}