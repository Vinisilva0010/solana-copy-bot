use crate::models::{RawIntent, RawTransactionEvent};

pub fn classify_pump_event(event: &RawTransactionEvent) -> RawIntent {
    if event.has_error {
        return RawIntent::Irrelevant;
    }

    let mut is_buy = false;
    let mut is_sell = false;

    for log in &event.logs {
        if log.as_str().contains("Instruction: Buy") {
            is_buy = true;
            break;
        } else if log.as_str().contains("Instruction: Sell") {
            is_sell = true;
            break;
        }
    }

    if is_buy {
        RawIntent::Buy { signature: event.signature.clone(), slot: event.slot }
    } else if is_sell {
        RawIntent::Sell { signature: event.signature.clone(), slot: event.slot }
    } else {
        RawIntent::Irrelevant
    }
}