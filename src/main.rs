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
    let mut event_count = 0;
    while let Some(event) = rx_classifier.recv().await {
        if !event.has_error {
            event_count += 1;
            // Apenas demonstrativo para provar que a fase 2 funciona sem congelar a tela
            if event_count % 100 == 0 {
                info!("100 novas transações válidas capturadas. Última: {}", event.signature);
            }
        }
    }

    Ok(())
}