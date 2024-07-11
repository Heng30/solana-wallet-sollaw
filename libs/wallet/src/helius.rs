use anyhow::Result;
use reqwest::{
    header::{HeaderMap, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct FileItem {
    pub uri: String,
    pub cdn_uri: String,

    #[serde(default)]
    pub mime: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Links {
    pub image: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Metadata {
    pub name: String,
    pub symbol: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Content {
    pub metadata: Metadata,
    pub links: Links,

    #[serde(default)]
    pub files: Vec<FileItem>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct AssetResult {
    pub id: String,
    pub content: Content,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BatchAssetData {
    pub result: Vec<AssetResult>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct AssetData {
    pub result: AssetResult,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BodyDataParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BodyData {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: BodyDataParams,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BatchBodyDataParams {
    pub ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BatchBodyData {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: BatchBodyDataParams,
}

const URL: &str = "https://mainnet.helius-rpc.com";
const HELIUS_FREE_API_KEY: &str = "335639a1-e34c-4b26-91bc-83b898c5a948";

pub async fn fetch_asset(id: String) -> Result<AssetData> {
    let client = Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let body_data = BodyData {
        jsonrpc: "2.0".to_string(),
        id: "text".to_string(),
        method: "getAsset".to_string(),
        params: BodyDataParams { id },
    };

    let assets = client
        .post(URL)
        .query(&[("api-key", HELIUS_FREE_API_KEY)])
        .timeout(Duration::from_secs(30))
        .headers(headers)
        .json(&body_data)
        .send()
        .await?
        .json::<AssetData>()
        .await?;

    Ok(assets)
}

pub async fn batch_fetch_assets(ids: Vec<String>) -> Result<BatchAssetData> {
    let client = Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let body_data = BatchBodyData {
        jsonrpc: "2.0".to_string(),
        id: "text".to_string(),
        method: "getAssetBatch".to_string(),
        params: BatchBodyDataParams { ids },
    };

    let assets = client
        .post(URL)
        .query(&[("api-key", HELIUS_FREE_API_KEY)])
        .timeout(Duration::from_secs(30))
        .headers(headers)
        .json(&body_data)
        .send()
        .await?
        .json::<BatchAssetData>()
        .await?;

    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    const USDC_MINT_ADDRESS: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    const USDT_MINT_ADDRESS: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

    #[tokio::test]
    async fn test_fetch_asset() -> Result<()> {
        let id = USDC_MINT_ADDRESS.to_string();
        let ret = fetch_asset(id).await?;
        println!("{:?}", ret);

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_fetch_assets() -> Result<()> {
        let ids = vec![USDC_MINT_ADDRESS.to_string(), USDT_MINT_ADDRESS.to_string()];
        let ret = batch_fetch_assets(ids).await?;
        println!("{:?}", ret);

        Ok(())
    }
}
