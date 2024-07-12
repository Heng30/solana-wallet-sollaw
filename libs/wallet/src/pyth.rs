use anyhow::Result;
use chrono::Utc;
use pyth_sdk_solana::state::SolanaPriceAccount;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{str::FromStr, time::Duration};

const URL: &str = "http:/pythnet.rpcpool.com";
const SOL_KEY: &str = "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG";

pub async fn sol(timeout: Option<u64>) -> Result<f64> {
    let connection = match timeout {
        Some(timeout) => RpcClient::new_with_timeout(URL.to_string(), Duration::from_secs(timeout)),
        None => RpcClient::new(URL.to_string()),
    };

    let current_time = Utc::now().timestamp();
    let price_key = Pubkey::from_str(SOL_KEY)?;
    let mut price_account = connection.get_account(&price_key).await?;
    let price_feed = SolanaPriceAccount::account_to_feed(&price_key, &mut price_account)?;

    price_feed
        .get_price_no_older_than(current_time, 60)
        .ok_or(anyhow::anyhow!("price unavailable"))
        .map(|v| v.price as f64 * 10_f64.powf(v.expo as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sol() -> Result<()> {
        let ret = sol(Some(60)).await;
        println!("{ret:?}");

        Ok(())
    }
}
