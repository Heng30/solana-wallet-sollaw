use super::tr::tr;
use crate::{
    db::{
        self,
        def::{TokenTileEntry, TOKENS_TABLE},
    },
    logic::message::{async_message_info, async_message_warn},
    message_info, message_success,
    slint_generatedAppWindow::{
        AppWindow, HomeIndex, Logic, SendTokenProps, Store, TokenTileEntry as UITokenTileEntry,
        TokenTileWithSwitchEntry as UITokenTileWithSwitchEntry, TokensSetting, Util,
    },
};
use anyhow::Result;
use slint::{ComponentHandle, Model, SharedString, VecModel, Weak};
use std::str::FromStr;
use uuid::Uuid;
use wallet::{
    network::{NetworkType, RpcUrlType},
    prelude::{lamports_to_sol, sol_to_lamports, Pubkey, LAMPORTS_PER_SOL},
    transaction::{self, DEFAULT_TIMEOUT_SECS, DEFAULT_TRY_COUNTS},
};

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
            _update_token_info(ui.as_weak(), network.clone(), entry);
        }
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
        .on_request_airdrop_sol(move |uuid, network, address| {
            _request_airdrop_sol(ui_handle.clone(), uuid, network, address);
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
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                match super::accounts::is_valid_password_in_secret_info(&password).await {
                    Err(e) => async_message_warn(ui_handle.clone(), format!("{e:?}")),
                    _ => {
                        if let Err(e) = if props.symbol == "SOL" {
                            _evaluate_sol_transaction_fee(ui_handle.clone(), password, props).await
                        } else {
                            _evaluate_spl_token_transaction_fee(ui_handle.clone(), password, props)
                                .await
                        } {
                            async_message_warn(ui_handle.clone(), format!("{e:?}"));
                        }
                    }
                }
            });
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_send_token(move |password, props| {
        let ui_handle = ui_handle.clone();
        tokio::spawn(async move {
            match super::accounts::is_valid_password_in_secret_info(&password).await {
                Err(e) => async_message_warn(ui_handle, format!("{e:?}")),
                _ => {
                    // TODO
                }
            }
        });
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

fn _update_token_info(ui: Weak<AppWindow>, network: SharedString, mut entry: UITokenTileEntry) {
    let rpc_url_ty = RpcUrlType::from_str(&network).unwrap_or(RpcUrlType::Main);

    if entry.symbol == "SOL" {
        let account_address = ui.unwrap().global::<Store>().get_current_account().pubkey;
        tokio::spawn(async move {
            if let Ok(lamports) =
                transaction::get_balance(rpc_url_ty, &account_address, Some(DEFAULT_TIMEOUT_SECS))
                    .await
            {
                entry.balance = wallet::util::lamports_to_sol_str(lamports).into();
                entry.balance_usdt = "TODO".into();
                _update_token_in_event_loop(ui, entry.clone());
                _update_token_db(entry);
            }
        });
        return;
    }

    if entry.mint_address.is_empty() {
        return;
    }

    tokio::spawn(async move {
        if let Ok(Some(ta)) = transaction::fetch_token_account(
            rpc_url_ty,
            &entry.mint_address,
            Some(DEFAULT_TIMEOUT_SECS),
        )
        .await
        {
            entry.balance = ta.token_amount.ui_amount_string.into();
            entry.balance_usdt = "$0.00".into();
            _update_token_in_event_loop(ui, entry.clone());
            _update_token_db(entry);
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
                                    symbol: mint_address.clone().into(), // TODO: Get the real token symbol
                                    account_address: account_address.clone(),
                                    mint_address: mint_address.clone().into(),
                                    balance: slint::format!("{}", token.amount()),
                                    balance_usdt: "$0.00".into(),
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

fn _request_airdrop_sol(
    ui_handle: Weak<AppWindow>,
    uuid: SharedString,
    network: SharedString,
    address: SharedString,
) {
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
                    _ => {
                        _ = slint::invoke_from_event_loop(move || {
                            let ui = ui_handle.unwrap();
                            let current_uuid = ui.global::<Store>().get_current_account().uuid;
                            if current_uuid == uuid {
                                ui.global::<Logic>()
                                    .invoke_update_account_balance(uuid, network, address);
                                message_success!(ui, tr("请求空投成功"));
                            }
                        });
                    }
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
                account_address: account_address.clone().into(),
                mint_address: String::default(),
                balance: "0.00".to_string(),
                balance_usdt: "$0.00".to_string(),
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
        sender.transaction_fee = slint::format!("{fee} SOL");
        sender.password = password;
        ui.global::<TokensSetting>().set_sender(sender);
        ui.global::<Store>()
            .set_current_home_index(HomeIndex::TransactionFee);
    });

    Ok(())
}

async fn _evaluate_spl_token_transaction_fee(
    ui: Weak<AppWindow>,
    password: SharedString,
    props: SendTokenProps,
) -> Result<()> {
    let _sender_pubkey = Pubkey::from_str(&props.send_address)?;
    let _recipient_pubkey = Pubkey::from_str(&props.recipient_address)?;
    let _amount = props.amount.parse::<f64>()?;

    Ok(())
    // TODO
}
