use crate::models::{Action, PaperTrade, TradingConfig};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

pub fn evaluate_action(action: &Action, config: &TradingConfig) -> Option<PaperTrade> {
    match action {
        Action::Buy { mint, amount_sol, tx_origin, wallet } => {
            // 1. Filtro de Whitelist
            if !config.target_wallets.contains(wallet) {
                debug!("Ignorado: Carteira {} não está na whitelist.", wallet);
                return None;
            }

            // 2. Cálculo de Tamanho da Posição (Risk Management)
            // Nossa estratégia copia o valor exato em SOL, limitado ao max_position_sol.
            let mut execute_sol = *amount_sol;

            if execute_sol < config.min_position_sol {
                debug!("Ignorado: Valor de entrada ({} SOL) abaixo do mínimo.", execute_sol);
                return None;
            }

            if execute_sol > config.max_position_sol {
                info!("Alerta de Risco: Reduzindo posição de {} para o limite máximo de {} SOL.", execute_sol, config.max_position_sol);
                execute_sol = config.max_position_sol;
            }

            // 3. Geração do Recibo (PAPER MODE)
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

            Some(PaperTrade {
                original_tx: tx_origin.clone(),
                mint: mint.clone(),
                side: "BUY".to_string(),
                execution_amount_sol: execute_sol,
                timestamp,
            })
        }
        Action::Sell { mint, tx_origin, wallet, .. } => {
            // Em V1 de cópia, vendas geralmente descarregam a posição toda ou proporcional.
            // Para o PAPER, vamos logar a intenção de venda integral se a carteira for alvo.
            if !config.target_wallets.contains(wallet) {
                return None;
            }

            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

            Some(PaperTrade {
                original_tx: tx_origin.clone(),
                mint: mint.clone(),
                side: "SELL".to_string(),
                execution_amount_sol: 0.0, // Vendas operam na balança de tokens, não definimos SOL de saída agora
                timestamp,
            })
        }
        _ => None,
    }
}