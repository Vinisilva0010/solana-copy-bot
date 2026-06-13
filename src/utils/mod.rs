use crate::models::{AppConfig, NetworkConfig, TelegramConfig, TradingConfig};
use std::env;

pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // Carrega apenas os blocos de configuração do TOML
    let settings = config::Config::builder()
        .add_source(config::File::with_name("config/default").required(false))
        .build()?;

    let network: NetworkConfig = settings.get("network")
        .unwrap_or(NetworkConfig { rpc_timeout_ms: 2000 });
        
    let trading: TradingConfig = settings.get("trading")
        .unwrap_or(TradingConfig {
            max_slippage_bps: 100,
            max_position_sol: 0.1,
            min_position_sol: 0.01,
            target_wallets: vec![],
        });

    // Montagem determinística: segredos exigem leitura direta do SO/Ambiente
    let mode_str = env::var("EXECUTION_MODE").unwrap_or_else(|_| "PAPER".to_string());
    let exec_mode = match mode_str.to_uppercase().as_str() {
        "LIVE" => crate::models::ExecutionMode::Live,
        "SIMULATED" => crate::models::ExecutionMode::Simulated,
        _ => crate::models::ExecutionMode::Paper,
    };

    let mut app_config = AppConfig {
        rpc_url_http: env::var("RPC_URL_HTTP").expect("FALTA VAR CRÍTICA: RPC_URL_HTTP no .env"),
        rpc_url_ws: env::var("RPC_URL_WS").expect("FALTA VAR CRÍTICA: RPC_URL_WS no .env"),
        execution_mode: exec_mode,
        wallet_path: env::var("WALLET_PATH").unwrap_or_else(|_| "bot_wallet.json".to_string()),
        telegram: None,
        network,
        trading,
    };

    if let (Ok(token), Ok(chat_id)) = (env::var("TELEGRAM_BOT_TOKEN"), env::var("TELEGRAM_CHAT_ID")) {
        app_config.telegram = Some(TelegramConfig { bot_token: token, chat_id });
    }

    Ok(app_config)
}