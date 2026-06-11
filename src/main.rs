pub mod classifier;
pub mod executor;
pub mod ingestion;
pub mod models;
pub mod strategy;
pub mod telemetry;
pub mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Iniciando sistema de inicialização...");

    let app_config = utils::load_config().expect("Falha crítica: Configuração base não encontrada ou inválida.");

    println!("Modo de execução configurado para: {}", app_config.execution_mode);
    println!("Slippage máxima: {} bps", app_config.trading.max_slippage_bps);

    Ok(())
}