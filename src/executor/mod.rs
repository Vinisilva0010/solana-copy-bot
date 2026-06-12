use crate::models::{PaperTrade, TradingConfig};
use reqwest::Client;
use serde::Serialize;
use solana_sdk::transaction::VersionedTransaction;
use std::error::Error;
use tracing::{debug, error, info};

const PUMP_PORTAL_LOCAL_API: &str = "https://pumpportal.fun/api/trade-local";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PumpPortalPayload {
    pub public_key: String,
    pub action: String,
    pub mint: String,
    pub amount: f64,
    pub denominated_in_sol: String,
    pub slippage: u32,
    pub priority_fee: f64,
    pub pool: String,
}

pub async fn build_transaction(
    http_client: &Client,
    trade: &PaperTrade,
    config: &TradingConfig,
    bot_public_key: &str,
) -> Result<VersionedTransaction, Box<dyn Error>> {
    let action_str = trade.side.to_lowercase(); // "buy" ou "sell"
    
    // Na PumpPortal, "true" (string) indica que o amount se refere a SOL, não à quantidade de tokens.
    // Como estamos no V1 focados em entradas via SOL, travamos como "true" para compras.
    // Para vendas totais, geralmente usamos amount "100%" e denominatedInSol "false".
   // Na PumpPortal, "true" (string) indica que o amount se refere a SOL...
    let (amount, denominated) = if action_str == "buy" {
        (trade.execution_amount_sol, "true")
    } else {
        (100.0, "false") // Placeholder para "vender 100% dos tokens"
    };

    let payload = PumpPortalPayload {
        public_key: bot_public_key.to_string(),
        action: action_str,
        mint: trade.mint.clone(),
        amount,
        denominated_in_sol: denominated.to_string(),
        slippage: config.max_slippage_bps / 100, // API espera % (ex: 100 bps = 1%)
        priority_fee: 0.0005, // Em produção, faremos fetch dinâmico do fee market
        pool: "pump".to_string(),
    };

    debug!("Solicitando construção de transação para a PumpPortal API...");

    let response = http_client
        .post(PUMP_PORTAL_LOCAL_API)
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        error!("Falha na PumpPortal API: {}", error_text);
        return Err(error_text.into());
    }

    // A API devolve o binário (bytes) da transação serializada.
    let tx_bytes = response.bytes().await?;
    
    // Desserializa os bytes puros para o objeto nativo do Solana SDK
    let transaction: VersionedTransaction = bincode::deserialize(&tx_bytes)?;
    
    info!("✅ Transação montada com sucesso! Instruções incluídas: {}", transaction.message.instructions().len());

    Ok(transaction)
}

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

pub async fn execute_transaction(
    rpc_client: &RpcClient,
    mut transaction: VersionedTransaction,
    keypair: &Keypair,
    mode: &str,
) -> Result<(), Box<dyn Error>> {
    // A transação construída pelo PumpPortal coloca a public_key enviada como fee_payer (índice 0)
    // Assinamos o buffer da mensagem e injetamos a assinatura criptográfica.
    let message_bytes = transaction.message.serialize();
    let signature = keypair.sign_message(&message_bytes);
    transaction.signatures[0] = signature;

    match mode.to_uppercase().as_str() {
        "PAPER" => {
            info!("[PAPER] Transação de {} assinada em memória. Nenhuma chamada RPC efetuada.", signature);
        }
        "SIMULATED" => {
            info!("[SIMULATED] Submetendo transação {} para simulação de rede...", signature);
            
            let config = RpcSimulateTransactionConfig {
                sig_verify: true, // Garante que a assinatura é válida (previne erro em LIVE)
                replace_recent_blockhash: false, // Usamos o blockhash otimizado já retornado pela PumpPortal
                commitment: Some(CommitmentConfig::processed()),
                ..Default::default()
            };

            let result = rpc_client.simulate_transaction_with_config(&transaction, config).await?;

            if let Some(err) = result.value.err {
                error!("[SIMULATED] Falha na simulação: {:?}", err);
                if let Some(logs) = result.value.logs {
                    for log in logs {
                        debug!("SimLog: {}", log);
                    }
                }
            } else {
                let units = result.value.units_consumed.unwrap_or(0);
                info!("[SIMULATED] Sucesso on-chain confirmado. Compute Units: {}", units);
            }
        }
        "LIVE" => {
            info!("[LIVE] Efetuando broadcast na mainnet. Assinatura: {}", signature);
            // Em fases futuras, este bloco será substituído por chamadas à API do Jito para envio de Bundles.
            let sig = rpc_client.send_transaction(&transaction).await?;
            info!("[LIVE] Transação enviada com sucesso: {}", sig);
        }
        _ => {
            error!("Modo de execução inválido: {}. Abortando.", mode);
        }
    }

    Ok(())
}