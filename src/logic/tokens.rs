use super::tr::tr;
use crate::{
    config,
    db::{
        self,
        def::{HistoryEntry, TokenTileEntry, TOKENS_TABLE},
    },
    logic::message::{async_message_info, async_message_success, async_message_warn},
    message_info, message_success, message_warn,
    slint_generatedAppWindow::{
        AppWindow, HomeIndex, Icons, LoadingStatus, Logic, PrioritizationFeeStatus, SendTokenProps,
        Store, TokenTileEntry as UITokenTileEntry,
        TokenTileWithSwitchEntry as UITokenTileWithSwitchEntry, TokensSetting,
        TransactionTileStatus, Util,
    },
};
use anyhow::{bail, Result};
use cutil::{http, time::local_now};
use once_cell::sync::Lazy;
use slint::{ComponentHandle, Image, Model, SharedString, VecModel, Weak};
use std::{collections::HashMap, fs, str::FromStr, sync::Mutex};
use uuid::Uuid;
use wallet::{
    helius::{self, AssetResult},
    network::{NetworkType, RpcUrlType},
    prelude::*,
    pyth,
    transaction::{self, SendLamportsProps, DEFAULT_TIMEOUT_SECS, DEFAULT_TRY_COUNTS},
};

static PRIORITIZATION_FEES: Lazy<Mutex<(u64, u64, u64)>> = Lazy::new(|| Mutex::new((0, 0, 0)));

static SOL_PRICE: Lazy<Mutex<f64>> = Lazy::new(|| Mutex::new(0.0));

static SPL_TOKENS_PRICE_INFO: Lazy<Mutex<HashMap<&'static str, SplTokenPriceInfo>>> =
    Lazy::new(|| {
        let mut prices = HashMap::new();

        for item in [
            (
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // mint_address
                "USDT",
                "3vxLXJqLqF3JG5TCbYycbKWRBbCJQLxQmBGCkyqEEefL",
            ),
            (
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "USDC",
                "Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD",
            ),
            (
                "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3",
                "PYTH",
                "nrYkQQQur7z8rYTST3G9GqATviK5SxTDkrqd21MW6Ue",
            ),
            (
                "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
                "JUP",
                "g6eRCbboSwK4tSWngn773RCMexr1APQr4uA9bGZBYfo",
            ),
            (
                "jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL",
                "JITO",
                "7yyaeuJ1GGtVBLT2z2xub5ZWYKaNhF28mj1RdV4VDFVk",
            ),
            (
                "hntyVP6YFm1Hg25TN9WGLqM12b8TQmcknKrdu1oxWux",
                "HNT",
                "7moA1i5vQUpfDwSpK6Pw9s56ahB7WFGidtbL2ujWrVvm",
            ),
            (
                "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
                "Bonk",
                "8ihFLu5FimgTQ1Unh4dVyEHUGodJ5gJQCrQf4KUVB9bN",
            ),
            (
                "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
                "RAY",
                "AnLf8tVYCM816gmBjiy8n53eXKKEDydT5piYjjQDPgTB",
            ),
        ] {
            prices.insert(
                item.0,
                SplTokenPriceInfo {
                    _symbol: item.1,
                    pyth_feed_id: item.2,
                    price: 0.0f64,
                },
            );
        }

        Mutex::new(prices)
    });

#[derive(Clone, Default, Debug)]
pub struct SplTokenPriceInfo {
    pub _symbol: &'static str,
    pub pyth_feed_id: &'static str,
    pub price: f64,
}

#[macro_export]
macro_rules! store_tokens_setting_entries {
    ($ui:expr) => {
        $ui.global::<TokensSetting>()
            .get_entries()
            .as_any()
            .downcast_ref::<VecModel<UITokenTileEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

#[macro_export]
macro_rules! store_tokens_setting_management_entries {
    ($ui:expr) => {
        $ui.global::<TokensSetting>()
            .get_management_entries()
            .as_any()
            .downcast_ref::<VecModel<UITokenTileWithSwitchEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

pub fn get_sol_price_cache() -> f64 {
    *SOL_PRICE.lock().unwrap()
}

pub fn set_sol_price_cache(v: f64) {
    *SOL_PRICE.lock().unwrap() = v;
}

pub async fn update_sol_price() {
    match pyth::sol(Some(transaction::DEFAULT_TIMEOUT_SECS)).await {
        Ok(price) => set_sol_price_cache(price),
        Err(e) => log::warn!("{e:?}"),
    }
}

pub fn get_spl_token_price(mint_address: &str) -> Option<f64> {
    SPL_TOKENS_PRICE_INFO
        .lock()
        .unwrap()
        .get(mint_address)
        .and_then(|item| Some(item.price))
}

pub async fn update_spl_token_price(mint_address: &str) {
    let pyth_feed_id = {
        match SPL_TOKENS_PRICE_INFO.lock().unwrap().get(mint_address) {
            None => return,
            Some(item) => item.pyth_feed_id.to_string(),
        }
    };

    match pyth::spl_token(&pyth_feed_id, Some(transaction::DEFAULT_TIMEOUT_SECS)).await {
        Ok(price) => {
            if let Some(entry) = SPL_TOKENS_PRICE_INFO.lock().unwrap().get_mut(mint_address) {
                entry.price = price;
            }
        }
        Err(e) => log::warn!("{e:?}"),
    }
}

pub async fn update_spl_tokens_price() {
    let feed_ids = {
        SPL_TOKENS_PRICE_INFO
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.to_string(), v.pyth_feed_id.to_string()))
            .collect::<Vec<_>>()
    };

    for item in feed_ids.into_iter() {
        tokio::spawn(async move {
            if let Ok(price) =
                pyth::spl_token(&item.1, Some(transaction::DEFAULT_TIMEOUT_SECS)).await
            {
                if let Some(entry) = SPL_TOKENS_PRICE_INFO
                    .lock()
                    .unwrap()
                    .get_mut(item.0.as_str())
                {
                    entry.price = price;
                }
            }
        });
    }
}

pub async fn update_prioritization_fees(network: &str) {
    let rpc_url_ty = RpcUrlType::from_str(network).unwrap_or(RpcUrlType::Main);
    match transaction::prioritization_fees(rpc_url_ty, Some(transaction::DEFAULT_TIMEOUT_SECS))
        .await
    {
        Err(e) => log::warn!("{e:?}"),
        Ok(v) => {
            *PRIORITIZATION_FEES.lock().unwrap() = v;
        }
    }
}

async fn get_from_db(network: &str, account_address: &str) -> Vec<UITokenTileEntry> {
    match db::entry::select_all(TOKENS_TABLE).await {
        Ok(items) => items
            .into_iter()
            .filter_map(
                |item| match serde_json::from_str::<TokenTileEntry>(&item.data) {
                    Ok(item) => {
                        if item.network == network && item.account_address == account_address {
                            Some(item.into())
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
            )
            .collect(),
        Err(e) => {
            log::warn!("{:?}", e);
            vec![]
        }
    }
}

fn get_entry(ui: &AppWindow, uuid: &str) -> Option<(usize, UITokenTileEntry)> {
    for (index, entry) in ui
        .global::<TokensSetting>()
        .get_entries()
        .iter()
        .enumerate()
    {
        if entry.uuid != uuid {
            continue;
        }

        return Some((index, entry));
    }

    None
}

fn get_entries(ui: &AppWindow, include_sol: bool) -> Vec<UITokenTileEntry> {
    ui.global::<TokensSetting>()
        .get_entries()
        .iter()
        .filter(|item| {
            if include_sol {
                true
            } else {
                item.symbol != "SOL"
            }
        })
        .collect()
}

async fn get_entries_by_account_address(account_address: &str) -> Vec<TokenTileEntry> {
    match db::entry::select_all(TOKENS_TABLE).await {
        Ok(items) => items
            .into_iter()
            .filter_map(
                |item| match serde_json::from_str::<TokenTileEntry>(&item.data) {
                    Ok(item) => {
                        if item.account_address == account_address {
                            Some(item)
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
            )
            .collect(),
        Err(e) => {
            log::warn!("{:?}", e);
            vec![]
        }
    }
}

pub fn init_tokens(ui: &AppWindow) {
    store_tokens_setting_entries!(ui).set_vec(vec![]);
    let network = ui.global::<Logic>().invoke_get_current_network();
    let account_address = ui.global::<Store>().get_current_account().pubkey;

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        let entries = get_from_db(&network, &account_address).await;

        let ui_handle = ui_handle.clone();
        _ = slint::invoke_from_event_loop(move || {
            let ui = ui_handle.unwrap();
            store_tokens_setting_entries!(ui).set_vec(entries);
        });

        update_sol_price().await;
        update_spl_tokens_price().await;
        update_prioritization_fees(&network).await;
    });
}

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_add_token(move |token| {
        let ui = ui_handle.unwrap();
        store_tokens_setting_entries!(ui).push(token.clone().into());
        _add_token(token.into());
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_add_sol_token_when_create_account(move |account_address| {
            _add_sol_token_when_create_account(&ui_handle.unwrap(), account_address);
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_remove_token(move |uuid| {
        let ui = ui_handle.unwrap();

        if let Some((index, _)) = get_entry(&ui, &uuid) {
            store_tokens_setting_entries!(ui).remove(index);
            _remove_entry(uuid);
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_remove_all_tokens(move || {
        store_tokens_setting_entries!(ui_handle.unwrap()).set_vec(vec![]);
        _remove_all_entry();
    });

    ui.global::<Logic>()
        .on_remove_tokens_when_remove_account(move |account_address| {
            _remove_tokens_when_remove_account(account_address);
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_update_tokens_info(move |network| {
        let ui = ui_handle.unwrap();

        message_info!(ui, tr("正在刷新..."));
        for entry in ui.global::<TokensSetting>().get_entries().iter() {
            _update_token_balance(&ui, network.clone(), entry.uuid.clone());
            _update_symbol_and_icon(&ui, entry.uuid);
        }
        message_success!(ui, tr("刷新完成"));
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_update_token_info(move |network, uuid| {
            let ui = ui_handle.unwrap();
            _update_token_balance(&ui, network, uuid.clone());
            _update_symbol_and_icon(&ui, uuid);
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_update_token_name(move |uuid, name| {
            if name.is_empty() {
                return;
            }

            let ui = ui_handle.unwrap();
            if let Some((index, mut entry)) = get_entry(&ui, &uuid) {
                entry.symbol = name;
                store_tokens_setting_entries!(ui).set_row_data(index, entry.clone());
                _update_token_db(entry);
            }
        });

    ui.global::<Logic>()
        .on_calculate_price_of_sol(move |sol_amount| {
            let amount = sol_amount.parse::<f64>().unwrap_or(0.0_f64);
            slint::format!("{:.3}", amount * get_sol_price_cache())
        });

    let ui_handle = ui.as_weak();
    ui.global::<Util>()
        .on_spl_token_icon(move |mint_address, icon_extension| {
            let ui = ui_handle.unwrap();
            let filepath = config::cache_dir().join(format!("{}.{}", mint_address, icon_extension));

            if filepath.exists() {
                Image::load_from_path(&filepath).unwrap_or(ui.global::<Icons>().get_token())
            } else {
                ui.global::<Icons>().get_token()
            }
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_refresh_tokens_management_entries(move |network, address| {
            _refresh_tokens_management_entries(ui_handle.clone(), network, address);
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_open_token_detail(move |network, mint_address| {
            let ui = ui_handle.unwrap();
            let url = NetworkType::from_str(&network)
                .unwrap_or(NetworkType::Main)
                .address_detail_url(&mint_address);

            ui.global::<Util>()
                .invoke_open_url("Default".into(), url.into());
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_request_airdrop_sol(move |network, address| {
            _request_airdrop_sol(ui_handle.clone(), network, address);
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_open_blockchain_network(move |network| {
            let ui = ui_handle.unwrap();
            let url = NetworkType::from_str(&network)
                .unwrap_or(NetworkType::Test)
                .homepage();
            ui.global::<Util>()
                .invoke_open_url("Default".into(), url.into());
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_evaluate_transaction_fee(move |password, props| {
            _evaluate_transaction_fee(ui_handle.clone(), password, props);
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_send_token(move |password, props, is_token_account_exist| {
            _send_token(ui_handle.clone(), password, props, is_token_account_exist);
        });

    ui.global::<Logic>()
        .on_is_valid_address(move |address| match Pubkey::from_str(&address) {
            Err(_) => tr("非法地址").into(),
            _ => SharedString::default(),
        });

    ui.global::<Logic>()
        .on_prioritization_fee(move |ty| match ty {
            PrioritizationFeeStatus::Slow => PRIORITIZATION_FEES.lock().unwrap().0 as i32,
            PrioritizationFeeStatus::Normal => PRIORITIZATION_FEES.lock().unwrap().1 as i32,
            PrioritizationFeeStatus::Fast => PRIORITIZATION_FEES.lock().unwrap().2 as i32,
        });

    ui.global::<Logic>()
        .on_is_valid_prioritization_fee(move |fee| {
            if fee.trim().is_empty() {
                return SharedString::default();
            }

            match fee.parse::<u64>() {
                Err(_) => tr("非法优先费用").into(),
                Ok(v) => {
                    let max_prioritization_fee = config::security_privacy().max_prioritization_fee;
                    if v <= max_prioritization_fee {
                        SharedString::default()
                    } else {
                        slint::format!(
                            "{}{}, {}",
                            tr("最大优先费用"),
                            max_prioritization_fee,
                            tr("请设置更大的优先费用")
                        )
                    }
                }
            }
        });
}

fn _add_token(entry: TokenTileEntry) {
    tokio::spawn(async move {
        _ = db::entry::insert(
            TOKENS_TABLE,
            &entry.uuid,
            &serde_json::to_string(&entry).unwrap(),
        )
        .await;
    });
}

fn _remove_entry(uuid: SharedString) {
    tokio::spawn(async move {
        _ = db::entry::delete(TOKENS_TABLE, &uuid).await;
    });
}

fn _remove_all_entry() {
    tokio::spawn(async move {
        _ = db::entry::delete_all(TOKENS_TABLE).await;
    });
}

fn _remove_tokens_when_remove_account(account_address: SharedString) {
    tokio::spawn(async move {
        let entries = get_entries_by_account_address(&account_address).await;
        for entry in entries.into_iter() {
            _ = db::entry::delete(TOKENS_TABLE, &entry.uuid).await;
        }
    });
}

fn _update_token_in_event_loop(ui: Weak<AppWindow>, entry: UITokenTileEntry) {
    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();
        if let Some((index, _)) = get_entry(&ui, &entry.uuid) {
            store_tokens_setting_entries!(ui).set_row_data(index, entry);
        }
    });
}

fn _update_token_balance(ui: &AppWindow, network: SharedString, uuid: SharedString) {
    let entry = get_entry(&ui, &uuid);
    if entry.is_none() {
        return;
    }
    let (_, entry) = entry.unwrap();

    let rpc_url_ty = RpcUrlType::from_str(&network).unwrap_or(RpcUrlType::Main);

    if entry.symbol == "SOL" {
        let account_address = ui.global::<Store>().get_current_account().pubkey;

        let ui_handle = ui.as_weak();
        tokio::spawn(async move {
            update_sol_price().await;

            if let Ok(lamports) =
                transaction::get_balance(rpc_url_ty, &account_address, Some(DEFAULT_TIMEOUT_SECS))
                    .await
            {
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui_handle.unwrap();
                    if let Some((index, mut entry)) = get_entry(&ui, &uuid) {
                        entry.balance = wallet::util::lamports_to_sol_str(lamports).into();

                        if get_sol_price_cache() > 0.0 {
                            entry.balance_usdt = slint::format!(
                                "${:.2}",
                                lamports_to_sol(lamports) * get_sol_price_cache()
                            );
                        }
                        store_tokens_setting_entries!(ui).set_row_data(index, entry.clone());
                        _update_token_db(entry);
                    }
                });
            }
        });
        return;
    }

    if entry.token_account_address.is_empty() {
        return;
    }

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        update_spl_token_price(&entry.mint_address).await;

        if let Ok(Some(ta)) = transaction::fetch_token_account(
            rpc_url_ty,
            &entry.token_account_address,
            Some(DEFAULT_TIMEOUT_SECS),
        )
        .await
        {
            _ = slint::invoke_from_event_loop(move || {
                let ui = ui_handle.unwrap();
                if let Some((index, mut entry)) = get_entry(&ui, &uuid) {
                    entry.balance = ta.token_amount.ui_amount_string.into();

                    if let Some(price) = get_spl_token_price(&entry.mint_address) {
                        if price > 0.0_f64 {
                            match entry.balance.parse::<f64>() {
                                Ok(balance) => {
                                    entry.balance_usdt = slint::format!("${:.2}", price * balance);
                                }
                                Err(e) => log::warn!("{e:?}"),
                            }
                        }
                    }

                    store_tokens_setting_entries!(ui).set_row_data(index, entry.clone());
                    _update_token_db(entry);
                }
            });
        }
    });
}

fn _update_token_db(entry: UITokenTileEntry) {
    tokio::spawn(async move {
        _ = db::entry::update(
            TOKENS_TABLE,
            &entry.uuid.clone(),
            &serde_json::to_string::<TokenTileEntry>(&entry.into()).unwrap(),
        )
        .await;
    });
}

async fn _update_fetch_assets(
    ui: Weak<AppWindow>,
    uuid: SharedString,
    item: AssetResult,
) -> Result<()> {
    let url = &item.content.links.image;
    let image_data = http::get_bytes(url, None).await?;

    http::file_extension(url)?.and_then(|extension| {
        let cache_file = config::cache_dir().join(format!("{}.{extension}", item.id));
        _ = fs::write(cache_file, image_data);

        _ = slint::invoke_from_event_loop(move || {
            let ui = ui.unwrap();

            if let Some((index, mut entry)) = get_entry(&ui, &uuid) {
                entry.symbol = item.content.metadata.symbol.into();
                entry.icon_extension = extension.into();
                store_tokens_setting_entries!(ui).set_row_data(index, entry.clone());
                _update_token_db(entry);
            }
        });

        None::<()>
    });

    Ok(())
}

fn _is_icon_exist(entry: &UITokenTileEntry) -> bool {
    let filepath =
        config::cache_dir().join(format!("{}.{}", entry.mint_address, entry.icon_extension));
    filepath.exists()
}

fn _update_symbol_and_icon(ui: &AppWindow, uuid: SharedString) {
    let entry = get_entry(&ui, &uuid);
    if entry.is_none() {
        return;
    }
    let (_, entry) = entry.unwrap();

    if _is_icon_exist(&entry) {
        return;
    }

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        match helius::fetch_asset(entry.mint_address.into()).await {
            Ok(item) => {
                if let Err(e) = _update_fetch_assets(ui_handle, entry.uuid, item.result).await {
                    log::warn!("{e:?}");
                }
            }
            Err(e) => log::warn!("{e:?}"),
        }
    });
}

fn _refresh_tokens_management_entries(
    ui_handle: Weak<AppWindow>,
    network: SharedString,
    address: SharedString,
) {
    let ui = ui_handle.unwrap();
    store_tokens_setting_management_entries!(ui).set_vec(vec![]);
    ui.global::<TokensSetting>()
        .set_management_entries_is_loading(true);

    let entries = get_entries(&ui, false);
    let current_network = ui.global::<Logic>().invoke_get_current_network();
    let account_address = ui.global::<Store>().get_current_account().pubkey;

    tokio::spawn(async move {
        let rpc_url_ty = RpcUrlType::from_str(&network).unwrap_or(RpcUrlType::Main);
        match transaction::fetch_account_tokens(rpc_url_ty, &address, Some(DEFAULT_TIMEOUT_SECS))
            .await
        {
            Ok(tokens) => {
                // Get the tokens not favorite
                let tokens = tokens
                    .into_iter()
                    .filter_map(|token| {
                        let mint_address = token.mint_address.to_string();
                        match entries
                            .iter()
                            .position(|entry| entry.mint_address == mint_address)
                        {
                            None => Some(UITokenTileWithSwitchEntry {
                                entry: UITokenTileEntry {
                                    uuid: Uuid::new_v4().to_string().into(),
                                    network: current_network.clone(),
                                    symbol: mint_address.clone().into(),
                                    icon_extension: SharedString::default(),
                                    account_address: account_address.clone(),
                                    token_account_address: token
                                        .token_account_address
                                        .to_string()
                                        .into(),
                                    mint_address: mint_address.clone().into(),
                                    balance: slint::format!("{}", token.amount()),
                                    balance_usdt: "$0.00".into(),
                                    decimals: token.decimals as i32,
                                },
                                checked: false,
                            }),
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>();

                let mut entries = entries
                    .into_iter()
                    .map(|item| UITokenTileWithSwitchEntry {
                        entry: item,
                        checked: true,
                    })
                    .collect::<Vec<_>>();

                entries.extend(tokens.into_iter());

                let ui_handle = ui_handle.clone();
                _ = slint::invoke_from_event_loop(move || {
                    store_tokens_setting_management_entries!(ui_handle.unwrap()).set_vec(entries);
                });
            }
            Err(e) => {
                async_message_warn(ui_handle.clone(), format!("{}. {e:?}", tr("获取Token失败")))
            }
        }

        _ = slint::invoke_from_event_loop(move || {
            ui_handle
                .unwrap()
                .global::<TokensSetting>()
                .set_management_entries_is_loading(false);
        });
    });
}

fn _request_airdrop_sol(ui_handle: Weak<AppWindow>, network: SharedString, address: SharedString) {
    tokio::spawn(async move {
        async_message_info(ui_handle.clone(), tr("请耐心等待..."));

        let ty = RpcUrlType::from_str(&network).unwrap_or(RpcUrlType::Test);
        match transaction::request_airdrop(
            ty.clone(),
            &address,
            LAMPORTS_PER_SOL / 10,
            Some(DEFAULT_TIMEOUT_SECS),
        )
        .await
        {
            Ok(sig) => {
                match transaction::wait_signature_confirmed(ty, &sig, DEFAULT_TRY_COUNTS, None)
                    .await
                {
                    Err(e) => {
                        async_message_warn(ui_handle, format!("{}. {e:?}", tr("请求空投失败")))
                    }
                    _ => async_message_success(ui_handle, tr("请求空投成功")),
                }
            }
            Err(e) => async_message_warn(ui_handle, format!("{}. {e:?}", tr("请求空投失败"))),
        }
    });
}

fn _add_sol_token_when_create_account(ui: &AppWindow, account_address: SharedString) {
    let current_network = ui.global::<Logic>().invoke_get_current_network();

    let entries = ["main", "test", "dev"]
        .into_iter()
        .map(|item| {
            let entry = TokenTileEntry {
                uuid: Uuid::new_v4().to_string(),
                network: item.to_string(),
                symbol: "SOL".to_string(),
                icon_extension: String::default(),
                account_address: account_address.clone().into(),
                token_account_address: String::default(),
                mint_address: String::default(),
                balance: "0.00".to_string(),
                balance_usdt: "$0.00".to_string(),
                decimals: 0,
            };

            if current_network == item {
                store_tokens_setting_entries!(ui).set_vec(vec![entry.clone().into()]);
            }

            entry
        })
        .collect::<Vec<_>>();

    tokio::spawn(async move {
        for entry in entries.into_iter() {
            _ = db::entry::insert(
                TOKENS_TABLE,
                &entry.uuid,
                &serde_json::to_string(&entry).unwrap(),
            )
            .await;
        }
    });
}

async fn _evaluate_sol_transaction_fee(
    ui: Weak<AppWindow>,
    password: SharedString,
    props: SendTokenProps,
) -> Result<()> {
    let rpc_url_ty =
        RpcUrlType::from_str(&props.network).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let sender_pubkey = Pubkey::from_str(&props.send_address)?;
    let recipient_pubkey = Pubkey::from_str(&props.recipient_address)?;
    let amount = props.amount.parse::<f64>()?;

    let lamports = sol_to_lamports(amount);
    let instructions =
        transaction::send_lamports_instruction(&sender_pubkey, &recipient_pubkey, lamports);
    let fee = transaction::evaluate_transaction_fee(
        rpc_url_ty,
        &instructions,
        &sender_pubkey,
        Some(DEFAULT_TIMEOUT_SECS),
    )
    .await?;

    let fee = lamports_to_sol(fee);
    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();
        let mut sender = ui.global::<TokensSetting>().get_sender();
        sender.transaction_fee = slint::format!("{fee}");
        sender.password = password;
        ui.global::<TokensSetting>().set_sender(sender);
    });

    Ok(())
}

async fn _evaluate_spl_token_transaction_fee(
    ui: Weak<AppWindow>,
    password: SharedString,
    props: SendTokenProps,
) -> Result<()> {
    let rpc_url_ty =
        RpcUrlType::from_str(&props.network).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let sender_pubkey = Pubkey::from_str(&props.send_address)?;
    let recipient_pubkey = Pubkey::from_str(&props.recipient_address)?;
    let mint_pubkey = Pubkey::from_str(&props.mint_address)?;

    let memo = if props.memo.trim().is_empty() {
        None
    } else {
        Some(props.memo.into())
    };

    let prioritization_fee = if !props.prioritization_fee.trim().is_empty() {
        let fee = props.prioritization_fee.trim().parse::<u64>()?;
        if fee == 0 {
            None
        } else {
            Some(fee)
        }
    } else {
        None
    };

    let info = super::accounts::get_secrect_info().await?;
    let sender_keypair =
        super::accounts::get_keypair(&password, &info.mnemonic, props.derive_index)?;

    if sender_pubkey.to_string() != sender_keypair.pubkey().to_string() {
        bail!(
            "can not match sender pubkey: [{}] with sender keypair",
            sender_pubkey.to_string()
        );
    }

    let sender_token_account_pubkey =
        transaction::derive_token_account_address(&sender_pubkey, &mint_pubkey);
    let recipient_token_account_pubkey =
        transaction::derive_token_account_address(&recipient_pubkey, &mint_pubkey);

    let amount = props.amount.parse::<f64>()?;
    let amount = (amount * 10_usize.pow(props.spl_token_decimals as u32) as f64) as u64;

    let send_spl_token_props = transaction::SendSplTokenProps {
        rpc_url_ty: rpc_url_ty.clone(),
        sender_keypair,
        sender_token_account_pubkey,
        recipient_token_account_pubkey,
        mint_pubkey,
        amount,
        decimals: props.spl_token_decimals as u8,
        timeout: Some(DEFAULT_TIMEOUT_SECS),
        is_wait_confirmed: true,
        memo,
        prioritization_fee,
    };
    let instructions = transaction::send_spl_token_instruction(&send_spl_token_props)?;
    let fee = transaction::evaluate_transaction_fee(
        rpc_url_ty.clone(),
        &instructions,
        &sender_pubkey,
        Some(DEFAULT_TIMEOUT_SECS),
    )
    .await?;

    let fee = lamports_to_sol(fee);
    match transaction::fetch_account_token(
        rpc_url_ty.clone(),
        &props.recipient_address,
        &props.mint_address,
        Some(DEFAULT_TIMEOUT_SECS),
    )
    .await?
    {
        Some(_) => {
            _ = slint::invoke_from_event_loop(move || {
                let ui = ui.unwrap();
                let mut sender = ui.global::<TokensSetting>().get_sender();
                sender.transaction_fee = slint::format!("{fee}");
                sender.password = password;
                sender.create_token_account_fee = SharedString::new();
                sender.is_token_account_exist = true;
                ui.global::<TokensSetting>().set_sender(sender);
            });
        }
        None => {
            _ = slint::invoke_from_event_loop(move || {
                let ui = ui.unwrap();
                let mut sender = ui.global::<TokensSetting>().get_sender();
                sender.transaction_fee = slint::format!("{fee}");
                sender.password = password;
                sender.create_token_account_fee = slint::format!(
                    "{}",
                    lamports_to_sol(transaction::DEFAULT_CREATE_TOKEN_ACCOUNT_RENT_LAMPORTS)
                );
                sender.is_token_account_exist = false;
                ui.global::<TokensSetting>().set_sender(sender);
            });
        }
    }

    Ok(())
}

async fn _send_sol(
    ui: Weak<AppWindow>,
    password: SharedString,
    props: SendTokenProps,
    history_uuid: String,
) -> Result<()> {
    let rpc_url_ty =
        RpcUrlType::from_str(&props.network).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let sender_pubkey = Pubkey::from_str(&props.send_address)?;
    let recipient_pubkey = Pubkey::from_str(&props.recipient_address)?;
    let amount = props.amount.parse::<f64>()?;

    let memo = if props.memo.trim().is_empty() {
        None
    } else {
        Some(props.memo.into())
    };

    let prioritization_fee = if !props.prioritization_fee.trim().is_empty() {
        let fee = props.prioritization_fee.trim().parse::<u64>()?;
        if fee == 0 {
            None
        } else {
            Some(fee)
        }
    } else {
        None
    };

    let info = super::accounts::get_secrect_info().await?;
    let sender_keypair =
        super::accounts::get_keypair(&password, &info.mnemonic, props.derive_index)?;

    if sender_pubkey.to_string() != sender_keypair.pubkey().to_string() {
        bail!(
            "can not match sender pubkey: [{}] with sender keypair",
            sender_pubkey.to_string()
        );
    }

    let send_props = SendLamportsProps {
        rpc_url_ty: rpc_url_ty.clone(),
        sender_keypair,
        recipient_pubkey,
        lamports: sol_to_lamports(amount),
        timeout: None,
        is_wait_confirmed: false,
        memo,
        prioritization_fee,
    };
    let signature = transaction::send_lamports(send_props).await?;

    let hash = signature.to_string();
    let history = HistoryEntry {
        uuid: history_uuid,
        network: props.network.clone().into(),
        hash,
        balance: props.amount.clone().into(),
        time: local_now("%Y-%m-%d %H:%M:%S"),
        status: TransactionTileStatus::Pending,
    };

    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();
        ui.global::<TokensSetting>()
            .invoke_set_signature(history.hash.clone().into());
        ui.global::<Logic>().invoke_add_history(history.into());
    });

    transaction::wait_signature_confirmed(rpc_url_ty, &signature, DEFAULT_TRY_COUNTS, None).await?;
    Ok(())
}

async fn _send_spl_token(
    ui: Weak<AppWindow>,
    password: SharedString,
    props: SendTokenProps,
    history_uuid: String,
    is_token_account_exist: bool,
) -> Result<()> {
    let rpc_url_ty =
        RpcUrlType::from_str(&props.network).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let sender_pubkey = Pubkey::from_str(&props.send_address)?;
    let recipient_pubkey = Pubkey::from_str(&props.recipient_address)?;
    let mint_pubkey = Pubkey::from_str(&props.mint_address)?;

    let memo = if props.memo.trim().is_empty() {
        None
    } else {
        Some(props.memo.into())
    };

    let prioritization_fee = if !props.prioritization_fee.trim().is_empty() {
        let fee = props.prioritization_fee.trim().parse::<u64>()?;
        if fee == 0 {
            None
        } else {
            Some(fee)
        }
    } else {
        None
    };

    let info = super::accounts::get_secrect_info().await?;
    let sender_keypair =
        super::accounts::get_keypair(&password, &info.mnemonic, props.derive_index)?;

    if sender_pubkey.to_string() != sender_keypair.pubkey().to_string() {
        bail!(
            "can not match sender pubkey: [{}] with sender keypair",
            sender_pubkey.to_string()
        );
    }

    let amount = props.amount.parse::<f64>()?;
    let amount = (amount * 10_usize.pow(props.spl_token_decimals as u32) as f64) as u64;

    let signature = if is_token_account_exist {
        let sender_token_account_pubkey =
            transaction::derive_token_account_address(&sender_pubkey, &mint_pubkey);
        let recipient_token_account_pubkey =
            transaction::derive_token_account_address(&recipient_pubkey, &mint_pubkey);

        let send_spl_token_props = transaction::SendSplTokenProps {
            rpc_url_ty: rpc_url_ty.clone(),
            sender_keypair,
            sender_token_account_pubkey,
            recipient_token_account_pubkey,
            mint_pubkey,
            amount,
            decimals: props.spl_token_decimals as u8,
            timeout: None,
            is_wait_confirmed: false,
            memo,
            prioritization_fee,
        };

        transaction::send_spl_token(send_spl_token_props).await?
    } else {
        let send_spl_token_props = transaction::SendSplTokenWithCreateProps {
            rpc_url_ty: rpc_url_ty.clone(),
            sender_keypair,
            recipient_pubkey: recipient_pubkey.clone(),
            mint_pubkey,
            amount,
            decimals: props.spl_token_decimals as u8,
            timeout: None,
            is_wait_confirmed: false,
            memo,
            prioritization_fee,
        };

        transaction::send_spl_token_with_create(send_spl_token_props).await?
    };

    let hash = signature.to_string();
    let history = HistoryEntry {
        uuid: history_uuid,
        network: props.network.clone().into(),
        hash,
        balance: props.amount.clone().into(),
        time: local_now("%Y-%m-%d %H:%M:%S"),
        status: TransactionTileStatus::Pending,
    };

    _ = slint::invoke_from_event_loop(move || {
        let ui = ui.unwrap();
        ui.global::<TokensSetting>()
            .invoke_set_signature(history.hash.clone().into());
        ui.global::<Logic>().invoke_add_history(history.into());
    });

    transaction::wait_signature_confirmed(rpc_url_ty.clone(), &signature, DEFAULT_TRY_COUNTS, None)
        .await?;

    Ok(())
}

fn _evaluate_transaction_fee(
    ui_handle: Weak<AppWindow>,
    password: SharedString,
    props: SendTokenProps,
) {
    tokio::spawn(async move {
        match super::accounts::is_valid_password_in_secret_info(&password).await {
            Err(e) => {
                let ui_handle = ui_handle.clone();
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui_handle.unwrap();
                    message_warn!(ui, format!("{e:?}"));
                });
            }
            _ => {
                let ui = ui_handle.clone();
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui.unwrap();
                    ui.global::<TokensSetting>()
                        .invoke_transaction_fee_loading_status(LoadingStatus::Loading);
                    ui.global::<Store>()
                        .set_current_home_index(HomeIndex::TransactionFee);
                });

                match if props.symbol == "SOL" {
                    _evaluate_sol_transaction_fee(ui_handle.clone(), password, props).await
                } else {
                    _evaluate_spl_token_transaction_fee(ui_handle.clone(), password, props).await
                } {
                    Err(e) => {
                        let ui_handle = ui_handle.clone();
                        _ = slint::invoke_from_event_loop(move || {
                            let ui = ui_handle.unwrap();
                            ui.global::<TokensSetting>()
                                .invoke_transaction_fee_loading_status(LoadingStatus::Fail);

                            message_warn!(ui, format!("{e:?}"));
                        });
                    }
                    _ => {
                        let ui_handle = ui_handle.clone();
                        _ = slint::invoke_from_event_loop(move || {
                            let ui = ui_handle.unwrap();
                            ui.global::<TokensSetting>()
                                .invoke_transaction_fee_loading_status(LoadingStatus::Success);
                        });
                    }
                }
            }
        }
    });
}

fn _send_token(
    ui_handle: Weak<AppWindow>,
    password: SharedString,
    props: SendTokenProps,
    is_token_account_exist: bool,
) {
    let network = props.network.clone();
    tokio::spawn(async move {
        update_prioritization_fees(&network).await;
    });

    tokio::spawn(async move {
        match super::accounts::is_valid_password_in_secret_info(&password).await {
            Err(e) => {
                let ui_handle = ui_handle.clone();
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui_handle.unwrap();
                    message_warn!(ui, format!("{e:?}"));
                });
            }
            _ => {
                let ui = ui_handle.clone();
                _ = slint::invoke_from_event_loop(move || {
                    let ui = ui.unwrap();
                    ui.global::<TokensSetting>()
                        .invoke_waiting_transaction_confirmed_loading_status(
                            LoadingStatus::Loading,
                        );
                    ui.global::<Store>()
                        .set_current_home_index(HomeIndex::WaitTransactionConfirmed);
                });

                let history_uuid = Uuid::new_v4().to_string();
                match if props.symbol == "SOL" {
                    _send_sol(
                        ui_handle.clone(),
                        password,
                        props.clone(),
                        history_uuid.clone(),
                    )
                    .await
                } else {
                    _send_spl_token(
                        ui_handle.clone(),
                        password,
                        props.clone(),
                        history_uuid.clone(),
                        is_token_account_exist,
                    )
                    .await
                } {
                    Err(e) => {
                        let ui_handle = ui_handle.clone();
                        _ = slint::invoke_from_event_loop(move || {
                            let ui = ui_handle.unwrap();
                            ui.global::<TokensSetting>()
                                .invoke_waiting_transaction_confirmed_loading_status(
                                    LoadingStatus::Fail,
                                );

                            ui.global::<Logic>().invoke_update_history_status(
                                history_uuid.into(),
                                TransactionTileStatus::Error,
                                true,
                            );

                            message_warn!(ui, format!("{}. {e:?}", tr("交易失败")));
                        });
                    }
                    _ => {
                        let ui_handle = ui_handle.clone();
                        _ = slint::invoke_from_event_loop(move || {
                            let ui = ui_handle.unwrap();
                            ui.global::<TokensSetting>()
                                .invoke_waiting_transaction_confirmed_loading_status(
                                    LoadingStatus::Success,
                                );

                            ui.global::<Logic>().invoke_update_history_status(
                                history_uuid.into(),
                                TransactionTileStatus::Success,
                                true,
                            );

                            ui.global::<Logic>()
                                .invoke_update_token_info(props.network.clone(), props.token_uuid);

                            message_success!(ui, tr("交易已经确认"));
                        });
                    }
                }
            }
        }
    });
}
