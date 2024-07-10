use super::network::{RpcUrlType, WssUrlType};
use anyhow::{bail, Context, Result};
use futures::stream::StreamExt;
use solana_account_decoder::{
    parse_token::UiTokenAccount, UiAccount, UiAccountEncoding, UiDataSliceConfig,
};
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
    rpc_response::Response,
};
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_instruction, system_program,
    transaction::Transaction,
};
use solana_transaction_status::UiTransactionEncoding;
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::state::Mint;
use std::{str::FromStr, string::ToString, time::Duration};

pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_TRY_COUNTS: u64 = 500;

#[derive(Default, Debug, Clone)]
pub struct AccountToken {
    pub token_account_address: Pubkey,
    pub mint_address: Pubkey,
    pub decimals: u8,
    amount: u64,
}

impl AccountToken {
    pub fn amount(&self) -> f64 {
        self.amount as f64 / 10_usize.pow(self.decimals as u32) as f64
    }
}

#[derive(Debug)]
pub struct SendLamportsProps {
    pub rpc_url_ty: RpcUrlType,
    pub sender_keypair: Keypair,
    pub recipient_pubkey: Pubkey,
    pub lamports: u64,
    pub timeout: Option<u64>,
    pub is_wait_confirmed: bool,
}

#[derive(Debug)]
pub struct SendSplTokenProps {
    pub rpc_url_ty: RpcUrlType,
    pub sender_keypair: Keypair,
    pub sender_token_account_pubkey: Pubkey,
    pub recipient_token_account_pubkey: Pubkey,
    pub mint_pubkey: Pubkey,
    pub amount: u64,
    pub decimals: u8,
    pub timeout: Option<u64>,
    pub is_wait_confirmed: bool,
}

#[derive(Debug)]
pub struct SendSplTokenWithCreateProps {
    pub rpc_url_ty: RpcUrlType,
    pub sender_keypair: Keypair,
    pub recipient_pubkey: Pubkey,
    pub mint_pubkey: Pubkey,
    pub amount: u64,
    pub decimals: u8,
    pub timeout: Option<u64>,
    pub is_wait_confirmed: bool,
}

#[derive(Debug)]
pub struct CreateSplTokenAccountProps {
    pub rpc_url_ty: RpcUrlType,
    pub payer_keypair: Keypair,
    pub wallet_pubkey: Pubkey,
    pub mint_pubkey: Pubkey,
    pub timeout: Option<u64>,
    pub is_wait_confirmed: bool,
}

#[derive(Debug)]
pub struct CreateOnlineAccountProps {
    pub rpc_url_ty: RpcUrlType,
    pub from_keypair: Keypair,
    pub new_account_keypair: Keypair,
    pub space: usize,
    pub rent_exemption_amount: Option<u64>,
    pub timeout: Option<u64>,
    pub is_wait_confirmed: bool,
}

#[derive(Debug)]
pub struct CreateOnlineAccountWithSeedProps {
    pub rpc_url_ty: RpcUrlType,
    pub base_keypair: Keypair,
    pub payer_keypair: Keypair,
    pub seed: String,
    pub space: usize,
    pub rent_exemption_amount: Option<u64>,
    pub timeout: Option<u64>,
    pub is_wait_confirmed: bool,
}

// return the fee of lamports
pub async fn evaluate_transaction_fee(
    rpc_url_ty: RpcUrlType,
    instructions: &[Instruction],
    payer: &Pubkey,
    timeout: Option<u64>,
) -> Result<u64> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    let recent_blockhash = connection
        .get_latest_blockhash()
        .await
        .with_context(|| "Get latest blockhash failed")?;

    let message = Message::new_with_blockhash(instructions, Some(payer), &recent_blockhash);

    connection
        .get_fee_for_message(&message)
        .await
        .with_context(|| "Get fee for Message failed")
}

pub async fn send_lamports(props: SendLamportsProps) -> Result<Signature> {
    let connection = match props.timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(props.rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(props.rpc_url_ty.to_string()),
    };

    let recent_blockhash = connection
        .get_latest_blockhash()
        .await
        .with_context(|| "Get latest blockhash failed")?;

    let instruction = system_instruction::transfer(
        &props.sender_keypair.pubkey(),
        &props.recipient_pubkey,
        props.lamports,
    );

    let message = Message::new(&[instruction], Some(&props.sender_keypair.pubkey()));
    let transaction = Transaction::new(&[props.sender_keypair], message, recent_blockhash);

    match props.is_wait_confirmed {
        true => connection.send_and_confirm_transaction(&transaction).await,
        false => connection.send_transaction(&transaction).await,
    }
    .with_context(|| "Send and confirm transation failed")
}

pub async fn create_spl_token_account(props: CreateSplTokenAccountProps) -> Result<Signature> {
    let connection = match props.timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(props.rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(props.rpc_url_ty.to_string()),
    };

    let recent_blockhash = connection
        .get_latest_blockhash()
        .await
        .with_context(|| "Get latest blockhash failed")?;

    let instruction = create_associated_token_account(
        &props.payer_keypair.pubkey(),
        &props.wallet_pubkey,
        &props.mint_pubkey,
        &spl_token::ID,
    );

    let message = Message::new(&[instruction], Some(&props.payer_keypair.pubkey()));
    let transaction = Transaction::new(&[props.payer_keypair], message, recent_blockhash);

    match props.is_wait_confirmed {
        true => connection.send_and_confirm_transaction(&transaction).await,
        false => connection.send_transaction(&transaction).await,
    }
    .with_context(|| "Send and confirm transation failed")
}

pub async fn send_spl_token(props: SendSplTokenProps) -> Result<Signature> {
    let connection = match props.timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(props.rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(props.rpc_url_ty.to_string()),
    };

    let token_info = fetch_token_info(
        props.rpc_url_ty,
        &props.mint_pubkey.to_string(),
        props.timeout,
    )
    .await?;
    let token_info = parse_token_info_data(token_info.data.as_slice())?;

    if token_info.decimals != props.decimals {
        bail!(
            "props decimal: {} != {}",
            props.decimals,
            token_info.decimals
        );
    }

    let instruction = spl_token::instruction::transfer_checked(
        &spl_token::ID,
        &props.sender_token_account_pubkey,
        &props.mint_pubkey,
        &props.recipient_token_account_pubkey,
        &props.sender_keypair.pubkey(),
        &[&props.sender_keypair.pubkey()],
        props.amount,
        props.decimals,
    )?;

    let recent_blockhash = connection
        .get_latest_blockhash()
        .await
        .with_context(|| "Get latest blockhash failed")?;

    let message = Message::new(&[instruction], Some(&props.sender_keypair.pubkey()));
    let transaction = Transaction::new(&[props.sender_keypair], message, recent_blockhash);

    match props.is_wait_confirmed {
        true => connection.send_and_confirm_transaction(&transaction).await,
        false => connection.send_transaction(&transaction).await,
    }
    .with_context(|| "Send and confirm transation failed")
}

pub async fn send_spl_token_with_create(props: SendSplTokenWithCreateProps) -> Result<Signature> {
    let connection = match props.timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(props.rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(props.rpc_url_ty.to_string()),
    };

    let token_info = fetch_token_info(
        props.rpc_url_ty,
        &props.mint_pubkey.to_string(),
        props.timeout,
    )
    .await?;
    let token_info = parse_token_info_data(token_info.data.as_slice())?;

    if token_info.decimals != props.decimals {
        bail!(
            "props decimal: {} != {}",
            props.decimals,
            token_info.decimals
        );
    }

    let create_instruction = create_associated_token_account(
        &props.sender_keypair.pubkey(),
        &props.recipient_pubkey,
        &props.mint_pubkey,
        &spl_token::ID,
    );

    let sender_token_account_pubkey =
        derive_account_token_address(&props.sender_keypair.pubkey(), &props.mint_pubkey);
    let recipient_token_account_pubkey =
        derive_account_token_address(&props.recipient_pubkey, &props.mint_pubkey);

    let send_instruction = spl_token::instruction::transfer_checked(
        &spl_token::ID,
        &sender_token_account_pubkey,
        &props.mint_pubkey,
        &recipient_token_account_pubkey,
        &props.sender_keypair.pubkey(),
        &[&props.sender_keypair.pubkey()],
        props.amount,
        props.decimals,
    )?;

    let recent_blockhash = connection
        .get_latest_blockhash()
        .await
        .with_context(|| "Get latest blockhash failed")?;

    let message = Message::new(
        &[create_instruction, send_instruction],
        Some(&props.sender_keypair.pubkey()),
    );
    let transaction = Transaction::new(&[props.sender_keypair], message, recent_blockhash);

    match props.is_wait_confirmed {
        true => connection.send_and_confirm_transaction(&transaction).await,
        false => connection.send_transaction(&transaction).await,
    }
    .with_context(|| "Send and confirm transation failed")
}

// return the balance of lamports
pub async fn get_balance(
    rpc_url_ty: RpcUrlType,
    pubkey: &str,
    timeout: Option<u64>,
) -> Result<u64> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };
    let pubkey = Pubkey::from_str(pubkey).with_context(|| format!("Invalid pubkey {pubkey}"))?;
    connection
        .get_balance(&pubkey)
        .await
        .with_context(|| format!("Get {pubkey} balance failed."))
}

pub async fn fetch_token_info(
    rpc_url_ty: RpcUrlType,
    mint_address: &str,
    timeout: Option<u64>,
) -> Result<Account> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    let mint_address_pubkey = Pubkey::from_str(mint_address)?;
    connection
        .get_account(&mint_address_pubkey)
        .await
        .with_context(|| format!("Get account {mint_address_pubkey} failed"))
}

pub fn parse_token_info_data(data: &[u8]) -> Result<Mint> {
    Mint::unpack_from_slice(data).with_context(|| "Mint unpack from slice failed")
}

pub async fn fetch_token_account(
    rpc_url_ty: RpcUrlType,
    token_account_address: &str,
    timeout: Option<u64>,
) -> Result<Option<UiTokenAccount>> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    let token_account_pubkey = Pubkey::from_str(token_account_address)?;
    connection
        .get_token_account(&token_account_pubkey)
        .await
        .with_context(|| format!("Get token account {token_account_address} failed"))
}

// derive the account token address locally
pub fn derive_account_token_address(
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
) -> Pubkey {
    get_associated_token_address(wallet_address, token_mint_address)
}

pub async fn fetch_account_token(
    rpc_url_ty: RpcUrlType,
    wallet_address: &str,
    mint_address: &str,
    timeout: Option<u64>,
) -> Result<AccountToken> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    let filters = Some(vec![
        RpcFilterType::Memcmp(Memcmp::new(
            0,
            MemcmpEncodedBytes::Base58(mint_address.to_string()),
        )),
        RpcFilterType::Memcmp(Memcmp::new(
            32,
            MemcmpEncodedBytes::Base58(wallet_address.to_string()),
        )),
        RpcFilterType::DataSize(165),
    ]);

    let accounts = connection
        .get_program_accounts_with_config(
            &spl_token::ID,
            RpcProgramAccountsConfig {
                filters,
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(connection.commitment()),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )
        .await
        .with_context(|| {
            format!("Get program accounts with config failed. wallet address: {wallet_address}")
        })?;

    if accounts.first().is_none() {
        bail!("Can't find the token account with the mint_address={mint_address}, wallet_address={wallet_address}");
    }

    let account = accounts.first().unwrap();
    let mut item = AccountToken::default();
    item.token_account_address = account.0;

    let mint_token_account = spl_token::state::Account::unpack_from_slice(
        account.1.data.as_slice(),
    )
    .with_context(|| {
        format!(
            "unpack from slice failed for {:?}",
            item.token_account_address
        )
    })?;
    item.mint_address = mint_token_account.mint;
    item.amount = mint_token_account.amount;

    let mint_account_data = connection
        .get_account_data(&mint_token_account.mint)
        .await
        .with_context(|| {
            format!(
                "Get account data failed for {:?}",
                item.token_account_address
            )
        })?;
    let mint = Mint::unpack_from_slice(mint_account_data.as_slice()).with_context(|| {
        format!(
            "Mint unpack from slice failed for {:?}",
            item.token_account_address
        )
    })?;

    item.decimals = mint.decimals;
    Ok(item)
}

pub async fn fetch_account_tokens(
    rpc_url_ty: RpcUrlType,
    address: &str,
    timeout: Option<u64>,
) -> Result<Vec<AccountToken>> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    let filters = Some(vec![
        RpcFilterType::Memcmp(Memcmp::new(
            32,
            MemcmpEncodedBytes::Base58(address.to_string()),
        )),
        RpcFilterType::DataSize(165),
    ]);

    let accounts = connection
        .get_program_accounts_with_config(
            &spl_token::ID,
            RpcProgramAccountsConfig {
                filters,
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(connection.commitment()),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )
        .await
        .with_context(|| {
            format!("Get program accounts with config failed. wallet address: {address}")
        })?;

    let mut items = vec![];
    for account in accounts.into_iter() {
        let mut item = AccountToken::default();
        item.token_account_address = account.0;

        let mint_token_account = spl_token::state::Account::unpack_from_slice(
            account.1.data.as_slice(),
        )
        .with_context(|| {
            format!(
                "unpack from slice failed for {:?}",
                item.token_account_address
            )
        })?;
        item.mint_address = mint_token_account.mint;
        item.amount = mint_token_account.amount;

        let mint_account_data = connection
            .get_account_data(&mint_token_account.mint)
            .await
            .with_context(|| {
                format!(
                    "Get account data failed for {:?}",
                    item.token_account_address
                )
            })?;
        let mint = Mint::unpack_from_slice(mint_account_data.as_slice()).with_context(|| {
            format!(
                "Mint unpack from slice failed for {:?}",
                item.token_account_address
            )
        })?;

        item.decimals = mint.decimals;

        items.push(item);
    }

    Ok(items)
}

pub async fn number_of_token_holders(
    rpc_url_ty: RpcUrlType,
    mint_address: &str,
    timeout: Option<u64>,
) -> Result<usize> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    let filters = Some(vec![
        RpcFilterType::Memcmp(Memcmp::new(
            0,
            MemcmpEncodedBytes::Base58(mint_address.to_string()),
        )),
        RpcFilterType::DataSize(165),
    ]);

    let accounts = connection
        .get_program_accounts_with_config(
            &spl_token::ID,
            RpcProgramAccountsConfig {
                filters,
                account_config: RpcAccountInfoConfig {
                    data_slice: Some(UiDataSliceConfig {
                        offset: 0,
                        length: 0,
                    }),
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(connection.commitment()),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )
        .await
        .with_context(|| {
            format!(
                "Get program accounts with config faild. token contract address: {mint_address}"
            )
        })?;

    Ok(accounts.len())
}

pub async fn request_airdrop(
    rpc_url_ty: RpcUrlType,
    address: &str,
    lamports: u64,
    timeout: Option<u64>,
) -> Result<Signature> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    let pubkey = Pubkey::from_str(address)
        .with_context(|| format!("Create pubkey from {address} failed"))?;

    connection
        .request_airdrop(&pubkey, lamports)
        .await
        .with_context(|| format!("Request airdrop for {address} failed"))
}

pub async fn wait_signature_confirmed(
    rpc_url_ty: RpcUrlType,
    signature: &Signature,
    try_counts: u64,
    timeout: Option<u64>,
) -> Result<u64> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    let mut counts = 1;
    loop {
        match connection.confirm_transaction(signature).await {
            Ok(true) => return Ok(counts),
            Ok(false) => {
                if counts >= try_counts {
                    bail!("send and confirm transaction for {try_counts} times, but all failed");
                }
                counts += 1;
            }
            Err(e) => {
                return Err(e).with_context(|| format!("Confirm transation: {signature} failed"))
            }
        }
    }
}

pub async fn is_signature_confirmed(
    rpc_url_ty: RpcUrlType,
    signature: &Signature,
    timeout: Option<u64>,
) -> Result<()> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    let transation = connection
        .get_transaction(signature, UiTransactionEncoding::Json)
        .await
        .with_context(|| format!("Get transacting {signature} failed"))?;

    match transation.transaction.meta {
        None => bail!("Can not find transation meta"),
        Some(meta) => match meta.err {
            Some(e) => bail!(e.to_string()),
            None => Ok(()),
        },
    }
}

// listening the wallet_address events including sending, receiving and other events
pub async fn account_subscribe(
    ws_url_ty: WssUrlType,
    wallet_address: &str,
    cb: impl Fn(Response<UiAccount>),
) -> Result<()> {
    let connection = PubsubClient::new(&ws_url_ty.to_string())
        .await
        .with_context(|| format!("New PubsubClient for {wallet_address} failed"))?;

    let mut receiver = connection
        .account_subscribe(
            &Pubkey::from_str(wallet_address)
                .with_context(|| format!("Generate pubkey from {wallet_address} failed"))?,
            Some(RpcAccountInfoConfig {
                encoding: None,
                data_slice: None,
                commitment: Some(CommitmentConfig::confirmed()),
                ..RpcAccountInfoConfig::default()
            }),
        )
        .await
        .with_context(|| format!("{wallet_address} subscribe failed"))?;

    while let Some(item) = receiver.0.next().await {
        cb(item);
    }

    bail!("Account subscribe exit")
}

pub async fn minimum_balance_for_rent_exemption(
    rpc_url_ty: RpcUrlType,
    space: usize,
    timeout: Option<u64>,
) -> Result<u64> {
    let connection = match timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(rpc_url_ty.to_string()),
    };

    connection
        .get_minimum_balance_for_rent_exemption(space)
        .await
        .with_context(|| {
            format!(
                "get_mininum_rent_exemption_amount for space {} failed",
                space
            )
        })
}

pub async fn create_online_account(props: CreateOnlineAccountProps) -> Result<Signature> {
    let connection = match props.timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(props.rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(props.rpc_url_ty.to_string()),
    };

    let min_rent_exemption_amount = connection
        .get_minimum_balance_for_rent_exemption(props.space)
        .await
        .with_context(|| {
            format!(
                "Get mininum_rent_exemption_amount for space {} failed",
                props.space
            )
        })?;

    let rent_exemption_amount = match props.rent_exemption_amount {
        Some(v) => {
            if v < min_rent_exemption_amount {
                bail!(format!("Insufficient rent_exemption_amount {v}. But request min_rent_exemption_amount {min_rent_exemption_amount}"));
            } else {
                v
            }
        }
        None => min_rent_exemption_amount,
    };

    let recent_blockhash = connection
        .get_latest_blockhash()
        .await
        .with_context(|| "get latest blockhash failed")?;

    let ix = system_instruction::create_account(
        &props.from_keypair.pubkey(),
        &props.new_account_keypair.pubkey(),
        rent_exemption_amount,
        props.space as u64,
        &props.from_keypair.pubkey(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&props.from_keypair.pubkey()),
        &[props.from_keypair, props.new_account_keypair],
        recent_blockhash,
    );

    match props.is_wait_confirmed {
        true => connection.send_and_confirm_transaction(&transaction).await,
        false => connection.send_transaction(&transaction).await,
    }
    .with_context(|| "Send and confirm transation failed")
}

pub async fn create_online_account_with_seed(
    props: CreateOnlineAccountWithSeedProps,
) -> Result<(Pubkey, Signature)> {
    let connection = match props.timeout {
        Some(timeout) => {
            RpcClient::new_with_timeout(props.rpc_url_ty.to_string(), Duration::from_secs(timeout))
        }
        None => RpcClient::new(props.rpc_url_ty.to_string()),
    };

    let program_id = system_program::id();
    let derived_pubkey =
        Pubkey::create_with_seed(&props.base_keypair.pubkey(), &props.seed, &program_id)
            .with_context(|| format!("create pubkey with seed: {}", props.seed))?;

    let min_rent_exemption_amount = connection
        .get_minimum_balance_for_rent_exemption(props.space)
        .await
        .with_context(|| {
            format!(
                "Get mininum_rent_exemption_amount for space {} failed",
                props.space
            )
        })?;

    let rent_exemption_amount = match props.rent_exemption_amount {
        Some(v) => {
            if v < min_rent_exemption_amount {
                bail!(format!("Insufficient rent_exemption_amount {v}. But request min_rent_exemption_amount {min_rent_exemption_amount}"));
            } else {
                v
            }
        }
        None => min_rent_exemption_amount,
    };

    let recent_blockhash = connection
        .get_latest_blockhash()
        .await
        .with_context(|| "get latest blockhash failed")?;

    let ix = system_instruction::create_account_with_seed(
        &props.payer_keypair.pubkey(),
        &derived_pubkey,
        &props.base_keypair.pubkey(),
        &props.seed,
        rent_exemption_amount,
        props.space as u64,
        &program_id,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&props.payer_keypair.pubkey()),
        &[props.payer_keypair, props.base_keypair],
        recent_blockhash,
    );

    let sig = match props.is_wait_confirmed {
        true => connection.send_and_confirm_transaction(&transaction).await,
        false => connection.send_transaction(&transaction).await,
    }
    .with_context(|| "Send and confirm transation failed")?;

    Ok((derived_pubkey, sig))
}

pub fn send_lamports_instruction(
    sender_pubkey: &Pubkey,
    recipient_pubkey: &Pubkey,
    lamports: u64,
) -> [Instruction; 1] {
    [system_instruction::transfer(
        sender_pubkey,
        recipient_pubkey,
        lamports,
    )]
}

pub fn send_spl_token_instruction(props: &SendSplTokenProps) -> Result<[Instruction; 1]> {
    Ok([spl_token::instruction::transfer_checked(
        &spl_token::ID,
        &props.sender_token_account_pubkey,
        &props.mint_pubkey,
        &props.recipient_token_account_pubkey,
        &props.sender_keypair.pubkey(),
        &[&props.sender_keypair.pubkey()],
        props.amount,
        props.decimals,
    )?])
}

pub fn create_spl_token_account_instruction(
    payer_address: &Pubkey,
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
) -> [Instruction; 1] {
    [create_associated_token_account(
        payer_address,
        wallet_address,
        token_mint_address,
        &spl_token::ID,
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::{Alphanumeric, DistString};
    use std::str::FromStr;

    const SENDER_KEYPAIR: &[u8] = &[
        82, 58, 225, 224, 31, 242, 176, 82, 123, 252, 190, 95, 252, 168, 178, 81, 25, 117, 29, 235,
        149, 159, 9, 226, 31, 41, 77, 251, 85, 225, 84, 142, 226, 85, 29, 242, 3, 193, 99, 113,
        185, 179, 128, 137, 235, 69, 204, 120, 224, 119, 51, 10, 73, 18, 165, 250, 218, 86, 201,
        14, 210, 151, 191, 40,
    ];

    const SENDER_WALLET_ADDRESS: &str = "GEWRDjNHTHdWZAzF8E4zHiqqnCEFWqhqHXnNKW2wdZN7";
    const RECIPIENT_WALLET_ADDRESS: &str = "5p6rcsWRHpkZmsusbuhsz9rgcPJWnbHcaDLgENTSUxY8";
    const TOKEN_ACCOUNT_ADDRESS_RECIPENT: &str = "AF6344WNH5rAkfyfx4wwEysNNH3gu7byrtD4eH7Mz294";
    const TOKEN_ACCOUNT_ADDRESS_SENDER: &str = "DDgMQe32RioZGf3RHSBSAiGTzwQXFf3C5XfahzaSg6eu";
    const USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS: &str =
        "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr";
    const NO_USDC_TOKEN_WALLET_ADDRESS: &str = "9njVzYc7S9CkyZiJDu85gJ1U7XURD1BSyEBzq8DLq2Fk";

    #[tokio::test]
    async fn test_evaluate_transaction_fee_send_sol() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let recipient_pubkey = Pubkey::from_str(RECIPIENT_WALLET_ADDRESS)?;

        let instructions =
            send_lamports_instruction(&sender_keypair.pubkey(), &recipient_pubkey, 100);

        let fee = evaluate_transaction_fee(
            RpcUrlType::Test,
            &instructions,
            &sender_keypair.pubkey(),
            None,
        )
        .await?;
        println!("fee: {fee}");

        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_transaction_fee_send_spl_token() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let sender_token_account_pubkey = Pubkey::from_str(TOKEN_ACCOUNT_ADDRESS_SENDER)?;
        let recipient_token_account_pubkey = Pubkey::from_str(TOKEN_ACCOUNT_ADDRESS_RECIPENT)?;
        let mint_pubkey = Pubkey::from_str(USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS)?;

        let props = SendSplTokenProps {
            rpc_url_ty: RpcUrlType::Test,
            sender_keypair,
            sender_token_account_pubkey,
            recipient_token_account_pubkey,
            mint_pubkey,
            amount: 1000_000,
            decimals: 6,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: true,
        };

        let instructions = send_spl_token_instruction(&props)?;
        let fee = evaluate_transaction_fee(
            RpcUrlType::Test,
            &instructions,
            &props.sender_keypair.pubkey(),
            None,
        )
        .await?;
        println!("fee: {fee}");

        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_transaction_fee_create_spl_token_account() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let new_address_keypair = Keypair::new();
        let token_mint_address = Pubkey::from_str(USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS)?;

        let instructions = create_spl_token_account_instruction(
            &sender_keypair.pubkey(),
            &new_address_keypair.pubkey(),
            &token_mint_address,
        );

        let fee = evaluate_transaction_fee(
            RpcUrlType::Test,
            &instructions,
            &sender_keypair.pubkey(),
            None,
        )
        .await?;
        println!("fee: {fee}");

        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_transaction_fee_create_account_and_send_spl_token() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let sender_token_account_pubkey = Pubkey::from_str(TOKEN_ACCOUNT_ADDRESS_SENDER)?;
        let token_mint_address = Pubkey::from_str(USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS)?;
        let mint_pubkey = Pubkey::from_str(USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS)?;

        let new_address_keypair = Keypair::new();
        let recipient_token_account_pubkey =
            derive_account_token_address(&new_address_keypair.pubkey(), &mint_pubkey);

        let create_instructions = create_spl_token_account_instruction(
            &sender_keypair.pubkey(),
            &new_address_keypair.pubkey(),
            &token_mint_address,
        );

        let props = SendSplTokenProps {
            rpc_url_ty: RpcUrlType::Test,
            sender_keypair: Keypair::from_bytes(&sender_keypair.to_bytes())?,
            sender_token_account_pubkey,
            recipient_token_account_pubkey,
            mint_pubkey,
            amount: 1000_000,
            decimals: 6,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: true,
        };
        let send_instructions = send_spl_token_instruction(&props)?;

        let fee = evaluate_transaction_fee(
            RpcUrlType::Test,
            &[create_instructions[0].clone(), send_instructions[0].clone()],
            &sender_keypair.pubkey(),
            None,
        )
        .await?;
        println!("fee: {fee}");

        Ok(())
    }

    #[tokio::test]
    async fn test_send_lamports() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let recipient_pubkey = Pubkey::from_str(RECIPIENT_WALLET_ADDRESS)?;
        let props = SendLamportsProps {
            rpc_url_ty: RpcUrlType::Test,
            sender_keypair,
            recipient_pubkey,
            lamports: 100,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: true,
        };

        let signature = send_lamports(props).await?;
        println!("{signature:?}");

        Ok(())
    }

    #[tokio::test]
    async fn test_create_spl_token_account() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let new_address_keypair = Keypair::new();
        let mint_pubkey = Pubkey::from_str(USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS)?;

        let props = CreateSplTokenAccountProps {
            rpc_url_ty: RpcUrlType::Test,
            payer_keypair: sender_keypair,
            wallet_pubkey: new_address_keypair.pubkey(),
            mint_pubkey,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: true,
        };
        let signature = create_spl_token_account(props).await?;
        println!("{signature:?}");

        Ok(())
    }

    #[tokio::test]
    async fn test_send_spl_token() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let sender_token_account_pubkey = Pubkey::from_str(TOKEN_ACCOUNT_ADDRESS_SENDER)?;
        let recipient_token_account_pubkey = Pubkey::from_str(TOKEN_ACCOUNT_ADDRESS_RECIPENT)?;
        let mint_pubkey = Pubkey::from_str(USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS)?;

        let props = SendSplTokenProps {
            rpc_url_ty: RpcUrlType::Test,
            sender_keypair,
            sender_token_account_pubkey,
            recipient_token_account_pubkey,
            mint_pubkey,
            amount: 1000_000,
            decimals: 6,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: true,
        };

        let signature = send_spl_token(props).await?;
        println!("{signature:?}");

        Ok(())
    }

    #[tokio::test]
    async fn test_send_spl_token_with_create() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let mint_pubkey = Pubkey::from_str(USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS)?;
        let recipient_keypair = Keypair::new();
        let recipient_pubkey = recipient_keypair.pubkey();

        let props = SendSplTokenWithCreateProps {
            rpc_url_ty: RpcUrlType::Test,
            sender_keypair,
            recipient_pubkey: recipient_pubkey.clone(),
            mint_pubkey,
            amount: 1_000_000,
            decimals: 6,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: true,
        };

        println!(
            "recipient_pubkey: {}",
            recipient_keypair.pubkey().to_string()
        );

        let signature = send_spl_token_with_create(props).await?;
        println!("{signature:?}");

        let info = fetch_account_token(
            RpcUrlType::Test,
            &recipient_pubkey.to_string(),
            USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS,
            Some(DEFAULT_TIMEOUT_SECS),
        )
        .await?;
        println!("{info:?}");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_balance() -> Result<()> {
        let lamports = get_balance(RpcUrlType::Test, SENDER_WALLET_ADDRESS, None).await?;
        println!("Balance: {lamports} lamports");

        Ok(())
    }

    #[test]
    fn test_derive_account_token_address() -> Result<()> {
        let recipient_address = Pubkey::from_str(RECIPIENT_WALLET_ADDRESS)?;
        let mint_pubkey = Pubkey::from_str(USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS)?;

        let address = derive_account_token_address(&recipient_address, &mint_pubkey);
        println!("{}", address.to_string());

        assert_eq!(address.to_string(), TOKEN_ACCOUNT_ADDRESS_RECIPENT);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_account_token_recipent() -> Result<()> {
        let ret = fetch_account_token(
            RpcUrlType::Test,
            RECIPIENT_WALLET_ADDRESS,
            USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS,
            Some(DEFAULT_TIMEOUT_SECS),
        )
        .await?;
        println!("{:?}", ret);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_account_token_no_usdc_wallet_address() -> Result<()> {
        let ret = fetch_account_token(
            RpcUrlType::Test,
            NO_USDC_TOKEN_WALLET_ADDRESS,
            USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS,
            Some(DEFAULT_TIMEOUT_SECS),
        )
        .await;
        println!("{:?}", ret);
        assert!(ret.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_account_tokens_recipent() -> Result<()> {
        let ret = fetch_account_tokens(
            RpcUrlType::Test,
            RECIPIENT_WALLET_ADDRESS,
            Some(DEFAULT_TIMEOUT_SECS),
        )
        .await?;
        println!("{:?}", ret);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_account_tokens_sender() -> Result<()> {
        let ret = fetch_account_tokens(
            RpcUrlType::Test,
            SENDER_WALLET_ADDRESS,
            Some(DEFAULT_TIMEOUT_SECS),
        )
        .await?;
        println!("{:?}", ret);

        Ok(())
    }

    #[tokio::test]
    async fn test_minimum_balance_for_rent_exemption() -> Result<()> {
        let ret = minimum_balance_for_rent_exemption(RpcUrlType::Main, 100, None).await?;
        println!("rent exemption amount: {:?} lamports", ret);

        Ok(())
    }

    #[tokio::test]
    async fn test_number_of_token_holders() -> Result<()> {
        let ret =
            number_of_token_holders(RpcUrlType::Test, USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS, None)
                .await?;
        println!("{:?}", ret);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_token_account() -> Result<()> {
        let ret =
            fetch_token_account(RpcUrlType::Test, TOKEN_ACCOUNT_ADDRESS_RECIPENT, None).await?;
        println!("{:?}", ret);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_token_info() -> Result<()> {
        let ret =
            fetch_token_info(RpcUrlType::Test, USDC_TOKEN_CONTRACT_TEST_NET_ADDRESS, None).await?;
        println!("{:?}", ret);

        let ret = parse_token_info_data(ret.data.as_slice())?;
        println!("{:?}", ret);

        Ok(())
    }

    #[tokio::test]
    async fn test_request_airdrop() -> Result<()> {
        let ret =
            request_airdrop(RpcUrlType::Test, SENDER_WALLET_ADDRESS, 100_000_000, None).await?;
        println!("request airdrop {:?}", ret);

        Ok(())
    }

    #[tokio::test]
    async fn test_wait_signature_confirmed() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let recipient_pubkey = Pubkey::from_str(RECIPIENT_WALLET_ADDRESS)?;
        let props = SendLamportsProps {
            rpc_url_ty: RpcUrlType::Test,
            sender_keypair,
            recipient_pubkey,
            lamports: 100,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: false,
        };

        let signature = send_lamports(props).await?;
        println!("{signature:?}");

        let ret = wait_signature_confirmed(RpcUrlType::Test, &signature, u64::MAX, None).await?;
        println!("wait_signature_confirmed try counts {ret}");

        Ok(())
    }

    #[tokio::test]
    async fn test_account_subscribe() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let recipient_pubkey = Pubkey::from_str(RECIPIENT_WALLET_ADDRESS)?;
        let props = SendLamportsProps {
            rpc_url_ty: RpcUrlType::Test,
            sender_keypair,
            recipient_pubkey,
            lamports: 100,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: false,
        };

        let signature = send_lamports(props).await?;
        println!("{signature:?}");

        account_subscribe(WssUrlType::Test, SENDER_WALLET_ADDRESS, |item| {
            println!("{item:?}");
        })
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_is_signature_confirmed_expect_failed() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let recipient_pubkey = Pubkey::from_str(RECIPIENT_WALLET_ADDRESS)?;
        let props = SendLamportsProps {
            rpc_url_ty: RpcUrlType::Test,
            sender_keypair,
            recipient_pubkey,
            lamports: 100,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: false,
        };

        let signature = send_lamports(props).await?;
        println!("{signature:?}");

        let ret = is_signature_confirmed(RpcUrlType::Test, &signature, None).await;
        println!("ret: {ret:?}");
        assert!(ret.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_is_signature_confirmed_expect_success() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let recipient_pubkey = Pubkey::from_str(RECIPIENT_WALLET_ADDRESS)?;
        let props = SendLamportsProps {
            rpc_url_ty: RpcUrlType::Test,
            sender_keypair,
            recipient_pubkey,
            lamports: 100,
            timeout: Some(DEFAULT_TIMEOUT_SECS),
            is_wait_confirmed: true,
        };

        let signature = send_lamports(props).await?;
        println!("{signature:?}");

        let ret = is_signature_confirmed(RpcUrlType::Test, &signature, None).await;
        println!("ret: {ret:?}");
        assert!(ret.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_create_online_account() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let new_account_keypair = Keypair::new();

        println!(
            "new account keypair: {}",
            new_account_keypair.pubkey().to_string()
        );

        let props = CreateOnlineAccountProps {
            rpc_url_ty: RpcUrlType::Test,
            from_keypair: sender_keypair,
            new_account_keypair,
            rent_exemption_amount: None,
            space: 100,
            timeout: None,
            is_wait_confirmed: true,
        };

        let sig = create_online_account(props).await?;
        println!("sig: {sig:?}");

        Ok(())
    }

    #[tokio::test]
    async fn test_create_online_account_with_seed() -> Result<()> {
        let sender_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let payer_keypair = Keypair::from_bytes(SENDER_KEYPAIR)?;
        let seed: String = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

        let props = CreateOnlineAccountWithSeedProps {
            rpc_url_ty: RpcUrlType::Test,
            base_keypair: sender_keypair,
            payer_keypair,
            seed,
            space: 100,
            rent_exemption_amount: None,
            timeout: None,
            is_wait_confirmed: true,
        };

        let (derived_pubkey, sig) = create_online_account_with_seed(props).await?;
        println!("derived_pubkey: {derived_pubkey:?}");
        println!("sig: {sig:?}");

        Ok(())
    }
}
