use std::{str::FromStr, string::ToString};

#[derive(Clone, Debug)]
pub enum NetworkType {
    Main,
    Test,
    Dev,
}

impl FromStr for NetworkType {
    type Err = String;

    fn from_str(ty: &str) -> Result<Self, Self::Err> {
        let ty = match ty.to_lowercase().as_str() {
            "main" => NetworkType::Main,
            "test" => NetworkType::Test,
            "dev" => NetworkType::Dev,
            _ => return Err(format!("Unknown Network type {ty}")),
        };

        Ok(ty)
    }
}

impl ToString for NetworkType {
    fn to_string(&self) -> String {
        String::from(match self {
            NetworkType::Main => "Main",
            NetworkType::Test => "Test",
            NetworkType::Dev => "Dev",
        })
    }
}

impl NetworkType {
    pub fn homepage(&self) -> String {
        let url = "https://explorer.solana.com/address";
        match *self {
            NetworkType::Main => format!("{url}"),
            NetworkType::Test => format!("{url}?cluster=testnet"),
            NetworkType::Dev => format!("{url}?cluster=devnet"),
        }
    }

    pub fn address_detail_url(&self, address: &str) -> String {
        let url = "https://explorer.solana.com/address";
        match *self {
            NetworkType::Main => format!("{url}/{address}"),
            NetworkType::Test => format!("{url}/{address}?cluster=testnet"),
            NetworkType::Dev => format!("{url}/{address}?cluster=devnet"),
        }
    }

    pub fn tx_detail_url(&self, hash: &str) -> String {
        let url = "https://explorer.solana.com/tx";
        match *self {
            NetworkType::Main => format!("{url}/{hash}"),
            NetworkType::Test => format!("{url}/{hash}?cluster=testnet"),
            NetworkType::Dev => format!("{url}/{hash}?cluster=devnet"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum RpcUrlType {
    Main,
    Test,
    Dev,
}

impl FromStr for RpcUrlType {
    type Err = String;

    fn from_str(ty: &str) -> Result<Self, Self::Err> {
        let ty = match ty.to_lowercase().as_str() {
            "main" => RpcUrlType::Main,
            "test" => RpcUrlType::Test,
            "dev" => RpcUrlType::Dev,
            _ => return Err(format!("Unknown Rpc type {ty}")),
        };

        Ok(ty)
    }
}

impl ToString for RpcUrlType {
    fn to_string(&self) -> String {
        String::from(match self {
            RpcUrlType::Main => "https://api.mainnet-beta.solana.com",
            RpcUrlType::Test => "https://api.testnet.solana.com",
            RpcUrlType::Dev => "https://api.devnet.solana.com",
        })
    }
}

#[derive(Clone, Debug)]
pub enum WssUrlType {
    Main,
    Test,
    Dev,
}

impl ToString for WssUrlType {
    fn to_string(&self) -> String {
        String::from(match self {
            WssUrlType::Main => "wss://api.mainnet-beta.solana.com",
            WssUrlType::Test => "wss://api.testnet.solana.com",
            WssUrlType::Dev => "wss://api.devnet.solana.com",
        })
    }
}
