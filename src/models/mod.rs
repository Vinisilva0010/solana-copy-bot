use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkConfig {
    pub rpc_timeout_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TradingConfig {
    pub max_slippage_bps: u32,
    pub max_position_sol: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub rpc_url_http: String,
    pub rpc_url_ws: String,
    pub execution_mode: String,
    pub telegram_bot_token: Option<String>,
    pub network: NetworkConfig,
    pub trading: TradingConfig,
}

#[derive(Debug, Clone)]
pub struct RawTransactionEvent {
    pub signature: String,
    pub logs: Vec<String>,
    pub has_error: bool,
}