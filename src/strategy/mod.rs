use crate::models::{EnrichedTrade, PaperTrade, TradingConfig};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn evaluate_action(trade: &EnrichedTrade, config: &TradingConfig) -> Option<PaperTrade> {
    if !config.target_wallets.contains(&trade.wallet) {
        return None;
    }

    let amount_to_execute = config.max_position_sol.min(trade.amount).max(config.min_position_sol);

    Some(PaperTrade {
        original_tx: trade.signature.clone(),
        mint: trade.mint.clone(),
        side: trade.side.clone(),
        execution_amount_sol: amount_to_execute,
        slot: trade.slot,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
    })
}