use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use tracing::debug;

pub async fn fetch_trade_details(
    http_client: &Client,
    rpc_url: &str,
    signature: &str,
) -> Result<(String, String, f64), Box<dyn Error>> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            signature,
            {
                "encoding": "jsonParsed",
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let res = http_client.post(rpc_url).json(&payload).send().await?;
    let json_data: Value = res.json().await?;

    if let Some(error) = json_data.get("error") {
        return Err(format!("Erro do RPC: {}", error).into());
    }

    let result = json_data.get("result")
        .ok_or("Transação não encontrada ou ainda não indexada pelo RPC")?;

    let wallet = result["transaction"]["message"]["accountKeys"][0]["pubkey"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    let mut mint = String::from("Unknown");
    if let Some(token_balances) = result["meta"]["postTokenBalances"].as_array() {
        if let Some(first_balance) = token_balances.first() {
            mint = first_balance["mint"].as_str().unwrap_or("Unknown").to_string();
        }
    }

    if mint == "Unknown" {
        if let Some(accounts) = result["transaction"]["message"]["accountKeys"].as_array() {
            if accounts.len() > 2 {
                mint = accounts[1]["pubkey"].as_str().unwrap_or("Unknown").to_string();
            }
        }
    }

    let mut amount_sol = 0.0;
    if let (Some(pre_balances), Some(post_balances)) = (
        result["meta"]["preBalances"].as_array(),
        result["meta"]["postBalances"].as_array(),
    ) {
        if !pre_balances.is_empty() && !post_balances.is_empty() {
            let pre_lamports = pre_balances[0].as_f64().unwrap_or(0.0);
            let post_lamports = post_balances[0].as_f64().unwrap_or(0.0);
            amount_sol = (pre_lamports - post_lamports).abs() / 1_000_000_000.0;
        }
    }

    debug!("Extrato on-chain | Wallet: {} | Mint: {} | Volume: {:.6} SOL", wallet, mint, amount_sol);

    Ok((wallet, mint, amount_sol))
}