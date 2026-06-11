use tokio::sync::mpsc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub mod classifier;
pub mod executor;
pub mod ingestion;
pub mod models;
pub mod strategy;
pub mod telemetry;
pub mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configura o sistema de logs. Em produção, mudaremos para Level::INFO.
    // Usar INFO no desenvolvimento também evita flood no terminal devido ao alto volume da Pump.fun.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let app_config = utils::load_config().expect("Falha ao carregar configuração.");
    info!("Executando bot. Modo: {}", app_config.execution_mode);

    // Cria o canal de comunicação (buffer de 10.000 mensagens para evitar backpressure)
    let (tx_ingestion, mut rx_classifier) = mpsc::channel::<models::RawTransactionEvent>(10000);

    // Inicializa a task de ingestão
    ingestion::start_stream(app_config.rpc_url_ws.clone(), tx_ingestion).await;

    // Loop temporário representando o Módulo Classifier (Passo 3 futuro)
    
    // Loop de Classificação e Estratégia
    while let Some(event) = rx_classifier.recv().await {
        if event.has_error { continue; }

        let action = classifier::classify_pump_event(&event);

        // Se o classificador identificar uma ação, passa para a estratégia
        if let Some(paper_trade) = strategy::evaluate_action(&action, &app_config.trading) {
            match paper_trade.side.as_str() {
                "BUY" => {
                    info!("📋 [PAPER TRADE] COMPRA APROVADA | Mint: {} | Valor: {} SOL | Tx Origem: {}", 
                        paper_trade.mint, paper_trade.execution_amount_sol, paper_trade.original_tx);
                    
                    // Futuro: Salvar em DB local ou passar para o Executor (Simulated/Live)
                }
                "SELL" => {
                    info!("📋 [PAPER TRADE] VENDA APROVADA | Mint: {} | Tx Origem: {}", 
                        paper_trade.mint, paper_trade.original_tx);
                }
                _ => {}
            }
        }
    }

    Ok(())
}