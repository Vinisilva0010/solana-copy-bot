use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkConfig {
    pub rpc_timeout_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TradingConfig {
    pub max_slippage_bps: u32,
    pub max_position_sol: f64,
    pub min_position_sol: f64,
    pub target_wallets: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub rpc_url_http: String,
    pub rpc_url_ws: String,
    pub execution_mode: ExecutionMode,
    pub wallet_path: String,
    pub telegram: Option<TelegramConfig>,
    pub network: NetworkConfig,
    pub trading: TradingConfig,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum ExecutionMode {
    Paper,
    Simulated,
    Live,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct RawTransactionEvent {
    pub signature: String,
    pub logs: Vec<String>,
    pub has_error: bool,
    pub slot: u64,
}

#[derive(Debug, Clone)]
pub enum RawIntent {
    Buy { signature: String, slot: u64 },
    Sell { signature: String, slot: u64 },
    Irrelevant,
}

#[derive(Debug, Clone)]
pub struct EnrichedTrade {
    pub side: TradeSide,
    pub signature: String,
    pub mint: String,
    pub wallet: String,
    pub amount: f64,
    pub slot: u64,
}

#[derive(Debug, Clone)]
pub struct PaperTrade {
    pub original_tx: String,
    pub mint: String,
    pub side: TradeSide,
    pub execution_amount_sol: f64,
    pub slot: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub execution_mode: String,
    pub original_tx: String,
    pub bot_tx: String,
    pub mint: String,
    pub amount_sol: f64,
    pub slot: Option<u64>,
    pub price: Option<f64>,
    pub mc_origin: Option<f64>,
    pub mc_bot: Option<f64>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStatus {
    Success,
    Failed,
    Expired,
    SimulatedSuccess,
    SimulatedFailed,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub mode: ExecutionMode,
    pub status: ExecutionStatus,
    pub signature: String,
    pub units_consumed: u64,
    pub slot: Option<u64>,
    pub logs: Vec<String>,
    pub error_msg: Option<String>,
}