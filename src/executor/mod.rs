use crate::models::{ExecutionResult, ExecutionStatus, PaperTrade, TradingConfig, ExecutionMode, TradeSide};
use reqwest::Client;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::VersionedTransaction;
use std::time::Duration;

pub async fn execute_trade(
    http_client: &Client,
    rpc_client: &RpcClient,
    paper_trade: &PaperTrade,
    trading_config: &TradingConfig,
    bot_keypair: &Keypair,
    execution_mode: &ExecutionMode,
) -> Result<ExecutionResult, Box<dyn std::error::Error + Send + Sync>> {
    
    let action_str = match paper_trade.side {
        TradeSide::Buy => "buy",
        TradeSide::Sell => "sell",
    };

    let denominated = match paper_trade.side {
        TradeSide::Buy => "true",
        TradeSide::Sell => "false",
    };

    let amount_val: serde_json::Value = if paper_trade.amount_payload == "100%" {
        json!("100%")
    } else {
        json!(paper_trade.amount_payload.parse::<f64>().unwrap_or(0.01))
    };

    let payload = json!({
        "publicKey": bot_keypair.pubkey().to_string(),
        "action": action_str,
        "mint": paper_trade.mint,
        "amount": amount_val,
        "denominatedInSol": denominated,
        "slippage": trading_config.max_slippage_bps,
        "priorityFee": 0.0001,
        "pool": "pump"
    });

    let res = http_client.post("https://pumpportal.fun/api/trade-local")
        .json(&payload)
        .send()
        .await?;

    let tx_bytes = res.bytes().await?;
    let mut tx: VersionedTransaction = bincode::deserialize(&tx_bytes)
        .map_err(|e| format!("Falha de desserialização da transação. Payload da PumpPortal falhou. Erro: {}", e))?;

    let (recent_blockhash, last_valid_block_height) = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await?;

    // ITEM 15: Política de Blockhash Explícita.
    // Descartamos o blockhash da API e forçamos o do nosso RPC para garantir 150 slots reais.
    tx.message.set_recent_blockhash(recent_blockhash);

    // ITEM 14: Validação rigorosa de Signer e Criptografia da VersionedTransaction
    let message_bytes = tx.message.serialize();
    
    let expected_signer = tx.message.static_account_keys().first()
        .ok_or("Payload corrompido: A API não retornou contas na transação.")?;
    
    if expected_signer != &bot_keypair.pubkey() {
        return Err(format!("Inconsistência Crítica: PumpPortal colocou {} como signer primário, mas o bot é {}", expected_signer, bot_keypair.pubkey()).into());
    }

    let signature = bot_keypair.sign_message(&message_bytes);
    
    if !signature.verify(bot_keypair.pubkey().as_ref(), &message_bytes) {
        return Err("Falha Matemática: A assinatura Ed25519 gerada é inválida para este bloco de bytes.".into());
    }

    if tx.signatures.is_empty() {
        tx.signatures.push(signature);
    } else {
        tx.signatures[0] = signature;
    }

    let sig_string = signature.to_string();

    // ITEM 17: Roteamento de Borda para Múltiplos Backends
    match *execution_mode {
        ExecutionMode::Live => {
            // Ponto de injeção arquitetural para MEV: 
            // if config.use_jito { return send_via_jito_bundle(...).await; }
            send_via_standard_rpc(rpc_client, tx, signature, sig_string, last_valid_block_height).await
        }
        ExecutionMode::Simulated | ExecutionMode::Paper => {
            simulate_via_standard_rpc(rpc_client, tx, sig_string, execution_mode.clone()).await
        }
    }
}

// ==========================================
// BACKENDS DE EXECUÇÃO ISOLADOS (Borda Arquitetural)
// ==========================================

async fn send_via_standard_rpc(
    rpc_client: &RpcClient,
    tx: VersionedTransaction,
    signature: solana_sdk::signature::Signature,
    sig_string: String,
    last_valid_block_height: u64,
) -> Result<ExecutionResult, Box<dyn std::error::Error + Send + Sync>> {
    let send_config = RpcSendTransactionConfig {
        skip_preflight: true,
        max_retries: Some(0),
        ..Default::default()
    };

    let mut current_status = ExecutionStatus::Failed; 
    let mut error_msg = None;

    if let Err(e) = rpc_client.send_transaction_with_config(&tx, send_config).await {
        tracing::warn!("Falha no broadcast da transação: {}. Tentando re-broadcast...", e);
    }

    loop {
        let current_height = rpc_client.get_block_height_with_commitment(CommitmentConfig::confirmed()).await?;
        if current_height > last_valid_block_height {
            current_status = ExecutionStatus::Expired;
            error_msg = Some("Blockhash expirado (LastValidBlockHeight ultrapassado)".to_string());
            break;
        }

        if let Ok(response) = rpc_client.get_signature_statuses(&[signature]).await {
            if let Some(Some(status)) = response.value.get(0) {
                if status.confirmation_status.is_some() {
                    if status.err.is_none() {
                        current_status = ExecutionStatus::Success;
                        break;
                    } else {
                        current_status = ExecutionStatus::Failed;
                        error_msg = Some(format!("Revertido pela rede: {:?}", status.err));
                        break;
                    }
                }
            }
        }

        if let Err(e) = rpc_client.send_transaction_with_config(&tx, send_config).await {
        tracing::warn!("Erro inicial no broadcast: {}", e);
       }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }

    Ok(ExecutionResult {
        mode: ExecutionMode::Live,
        status: current_status,
        signature: sig_string,
        units_consumed: 0,
        slot: None,
        logs: vec![],
        error_msg,
    })
}

async fn simulate_via_standard_rpc(
    rpc_client: &RpcClient,
    tx: VersionedTransaction,
    sig_string: String,
    original_mode: ExecutionMode,
) -> Result<ExecutionResult, Box<dyn std::error::Error + Send + Sync>> {
    // ITEM 16: Fidelidade Absoluta na Simulação (preservado)
    let sim_config = RpcSimulateTransactionConfig {
        sig_verify: true, 
        replace_recent_blockhash: false, 
        commitment: Some(CommitmentConfig::confirmed()),
        ..Default::default()
    };

    let sim_result = rpc_client.simulate_transaction_with_config(&tx, sim_config).await?;
    let val = sim_result.value;

    let status = if val.err.is_none() {
        ExecutionStatus::SimulatedSuccess
    } else {
        ExecutionStatus::SimulatedFailed
    };

    Ok(ExecutionResult {
        mode: original_mode,
        status,
        signature: sig_string,
        units_consumed: val.units_consumed.unwrap_or(0),
        slot: None,
        logs: val.logs.unwrap_or_default(),
        error_msg: val.err.map(|e| format!("{:?}", e)),
    })
}