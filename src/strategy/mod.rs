use crate::models::{EnrichedTrade, PaperTrade, TradingConfig, TradeSide};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn evaluate_action(trade: &EnrichedTrade, config: &TradingConfig) -> Option<PaperTrade> {
    if !config.target_wallets.contains(&trade.wallet) {
        tracing::debug!("Evento ignorado: wallet {} não está na whitelist.", trade.wallet);
        return None;
    }

    let (payload, db_sol) = match trade.side {
        TradeSide::Buy => {
            let amt = config.max_position_sol.min(trade.amount).max(config.min_position_sol);
            (amt.to_string(), amt)
        }
        TradeSide::Sell => {
            ("100%".to_string(), 0.0)
        }
    };

    Some(PaperTrade {
        original_tx: trade.signature.clone(),
        mint: trade.mint.clone(),
        side: trade.side.clone(),
        amount_payload: payload,
        amount_sol_db: db_sol,
        slot: trade.slot,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
    })
}