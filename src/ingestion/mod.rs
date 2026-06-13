use crate::models::RawTransactionEvent;
use futures_util::StreamExt;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_rpc_client_api::config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub async fn start_stream(ws_url: String, tx_channel: mpsc::Sender<RawTransactionEvent>) {
    tokio::spawn(async move {
        let mut backoff_secs = 1;
        
        loop {
            info!("Tentando conectar ao WebSocket RPC: {}", ws_url);
            
            match PubsubClient::new(&ws_url).await {
                Ok(client) => {
                    info!("Conexão WebSocket estabelecida com sucesso.");
                    backoff_secs = 1; 
                    
                    let filter = RpcTransactionLogsFilter::Mentions(vec![
                        "6EF8rrecthR5Dkzon8Nwu78hRvfX9PNXTxLjcjBgTFaM".to_string(), 
                    ]);
                    let config = RpcTransactionLogsConfig {
                        commitment: Some(CommitmentConfig::processed()),
                    };

                    match client.logs_subscribe(filter, config).await {
                        Ok((mut stream, _unsub)) => {
                            info!("Subscrição de logs ativada para o programa Pump.fun.");
                            
                            while let Some(log_info) = stream.next().await {
                                let event = RawTransactionEvent {
                                    signature: log_info.value.signature.clone(),
                                    logs: log_info.value.logs.clone(),
                                    has_error: log_info.value.err.is_some(),
                                    slot: log_info.context.slot,
                                };
                                
                                if tx_channel.send(event).await.is_err() {
                                    error!("Canal de ingestão corrompido. Encerrando worker de rede.");
                                    return;
                                }
                            }
                            warn!("A stream de logs foi finalizada pelo servidor RPC. Iniciando reconexão...");
                        }
                        Err(e) => {
                            error!("Falha ao assinar logs na Helius: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Falha na conexão com o WebSocket: {}", e);
                }
            }

            warn!("Rede instável. Aguardando {} segundos para a próxima tentativa...", backoff_secs);
            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            
            backoff_secs = std::cmp::min(backoff_secs * 2, 30);
        }
    });
}