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

#[derive(Debug, Clone)]
pub enum Action {
    Buy {
        mint: String,
        amount_sol: f64,
        tx_origin: String,
        wallet: String,
    },
    Sell {
        mint: String,
        amount_tokens: f64,
        tx_origin: String,
        wallet: String,
    },
    PartialSell {
        percentage: f64,
        tx_origin: String,
        wallet: String,
    },
    Transfer {
        direction: String,
        amount: f64,
        tx_origin: String,
    },
    Irrelevant,
}

#[derive(Debug, Clone)]
pub struct PaperTrade {
    pub original_tx: String,
    pub mint: String,
    pub side: String,
    pub execution_amount_sol: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub execution_mode: String,
    pub original_tx: String,
    pub bot_tx: String,
    pub mint: String,
    pub amount_sol: f64,
    pub slot: u64,
    pub price: f64,
    pub mc_origin: f64,
    pub mc_bot: f64,
    pub timestamp: u64,
}