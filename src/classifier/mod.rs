use crate::models::{Action, RawTransactionEvent};


pub fn classify_pump_event(event: &RawTransactionEvent) -> Action {
    if event.has_error {
        return Action::Irrelevant;
    }

    let mut is_buy = false;
    let mut is_sell = false;

    // Busca heurística rápida no vetor de logs
    // Evitamos Regex aqui porque a compilação de regex em loops de HFT destrói o throughput da CPU.
    for log in &event.logs {
        if log.contains("Instruction: Buy") {
            is_buy = true;
            break;
        } else if log.contains("Instruction: Sell") {
            is_sell = true;
            break;
        }
    }

    // Nota de arquitetura: Os valores abaixo são mockados.
    // Em produção, ao confirmar is_buy, o bot usaria a `signature` para buscar a transação
    // completa via RPC (`get_transaction`) ou decodificaria o log base64 da Pump.fun 
    // para extrair o mint e os amounts reais sem chamadas de rede adicionais.
    if is_buy {
        Action::Buy {
            mint: "ExtracaoPendente".to_string(),
            amount_sol: 0.0,
            tx_origin: event.signature.clone(),
        }
    } else if is_sell {
        Action::Sell {
            mint: "ExtracaoPendente".to_string(),
            amount_tokens: 0.0,
            tx_origin: event.signature.clone(),
        }
    } else {
        Action::Irrelevant
    }
}