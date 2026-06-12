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

    if is_buy {
        Action::Buy {
            mint: String::new(),
            amount_sol: 0.1, // O volume real em SOL será decodificado na Parte 11
            tx_origin: event.signature.clone(),
            wallet: String::new(),
        }
    } else if is_sell {
        Action::Sell {
            mint: String::new(),
            amount_tokens: 100.0,
            tx_origin: event.signature.clone(),
            wallet: String::new(),
        }
    } else {
        Action::Irrelevant
    }
}