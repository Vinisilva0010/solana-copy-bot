use crate::models::{Action, RawTransactionEvent};

pub fn classify_pump_event(event: &RawTransactionEvent) -> Action {
    if event.has_error {
        return Action::Irrelevant;
    }

    let mut is_buy = false;
    let mut is_sell = false;

    for log in &event.logs {
        if log.contains("Instruction: Buy") {
            is_buy = true;
            break;
        } else if log.contains("Instruction: Sell") {
            is_sell = true;
            break;
        }
    }

    // Mock: Em produção, o signer e o valor serão extraídos da transação decodificada.
    let mock_target_wallet = "TargetWallet111111111111111111111111111111".to_string();

    if is_buy {
        Action::Buy {
            mint: "ExtracaoPendente".to_string(),
            amount_sol: 0.1, // Valor mockado da compra original
            tx_origin: event.signature.clone(),
            wallet: mock_target_wallet,
        }
    } else if is_sell {
        Action::Sell {
            mint: "ExtracaoPendente".to_string(),
            amount_tokens: 1000.0,
            tx_origin: event.signature.clone(),
            wallet: mock_target_wallet,
        }
    } else {
        Action::Irrelevant
    }
}