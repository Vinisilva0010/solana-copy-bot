use crate::models::RawTransactionEvent;
use futures::StreamExt;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_rpc_client_api::config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

// Program ID oficial da Pump.fun
const PUMP_FUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfX9PNnnu9dZdzmpTqz";

pub async fn start_stream(ws_url: String, tx_channel: mpsc::Sender<RawTransactionEvent>) {
    // Spawna uma task isolada para não bloquear a thread principal
    tokio::spawn(async move {
        let pubsub_result = PubsubClient::new(&ws_url).await;
        
        let pubsub = match pubsub_result {
            Ok(client) => client,
            Err(e) => {
                error!("Falha crítica de conexão RPC WebSocket: {}", e);
                return;
            }
        };

        let filter = RpcTransactionLogsFilter::Mentions(vec![PUMP_FUN_PROGRAM_ID.to_string()]);
        
        // Uso de Processed para garantir latência mínima
        let config = RpcTransactionLogsConfig {
            commitment: Some(CommitmentConfig::processed()),
        };

        let (mut stream, _unsub) = match pubsub.logs_subscribe(filter, config).await {
            Ok(s) => s,
            Err(e) => {
                error!("Falha ao assinar logs da rede: {}", e);
                return;
            }
        };

        info!("Subscrição de logs ativada para o programa Pump.fun.");

        while let Some(response) = stream.next().await {
            let log_data = response.value;
            
            let event = RawTransactionEvent {
                signature: log_data.signature.clone(),
                logs: log_data.logs,
                has_error: log_data.err.is_some(),
            };

            debug!(signature = %event.signature, "Log capturado on-chain");

            // Envia para o canal. Se o receiver cair, interrompe a task de ingestão de forma limpa.
            if tx_channel.send(event).await.is_err() {
                error!("Canal MPSC fechado. Interrompendo ingestão.");
                break;
            }
        }
        
        error!("Stream do WebSocket encerrado inesperadamente pelo provider.");
    });
}