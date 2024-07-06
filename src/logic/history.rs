use super::tr::tr;
use crate::{
    config,
    db::{
        self,
        def::{HistoryEntry, HISTORY_TABLE},
    },
    logic::message::async_message_success,
    message_info, message_success, message_warn,
    slint_generatedAppWindow::{
        AppWindow, HistorySetting, Logic, TransactionTileEntry as UIHistoryEntry,
        TransactionTileStatus, Util,
    },
};
use anyhow::Result;
use cutil::time::local_now;
use slint::{ComponentHandle, Model, SharedString, VecModel};
use std::str::FromStr;
use uuid::Uuid;
use wallet::{
    network::{NetworkType, RpcUrlType},
    prelude::*,
    transaction::{self, is_signature_confirmed},
};

#[macro_export]
macro_rules! store_history_entries {
    ($ui:expr) => {
        $ui.global::<HistorySetting>()
            .get_entries()
            .as_any()
            .downcast_ref::<VecModel<UIHistoryEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

async fn add_mock_entries_to_db(count: u32) -> Result<()> {
    let row = db::entry::row_counts(HISTORY_TABLE).await.unwrap_or(0);
    let count = u32::max(0, count - row as u32);

    for index in 0..count {
        let entry = HistoryEntry {
            uuid: Uuid::new_v4().to_string(),
            network: NetworkType::Test.to_string(),
            hash: "3TLjgoAExvGWPrcPobxJMGrYiKQaHbrQzeG1aCz6A7D4Uz3BT6gXkmD4WYEcamK3aEpRuqoQL2Np64YZBkC2hUwF".to_string(),
            balance: "12.54 lamports".into(),
            time: local_now("%y-%m-%d %H:%M:%S"),
            status: if index % 2 == 0 {
                TransactionTileStatus::Pending
            } else {
                TransactionTileStatus::Error
            },
        };

        _ = db::entry::insert(
            HISTORY_TABLE,
            &entry.uuid,
            &serde_json::to_string(&entry).unwrap(),
        )
        .await;
    }

    Ok(())
}

async fn get_from_db(ty: NetworkType) -> Vec<UIHistoryEntry> {
    let network = ty.to_string();

    match db::entry::select_all(HISTORY_TABLE).await {
        Ok(items) => items
            .into_iter()
            .filter_map(
                |item| match serde_json::from_str::<HistoryEntry>(&item.data).ok() {
                    None => None,
                    Some(item) => {
                        if item.network == network {
                            Some(item.into())
                        } else {
                            None
                        }
                    }
                },
            )
            .rev()
            .collect(),
        Err(e) => {
            log::warn!("{:?}", e);
            vec![]
        }
    }
}

fn get_entry(ui: &AppWindow, uuid: &str) -> Option<(usize, UIHistoryEntry)> {
    for (index, item) in ui
        .global::<HistorySetting>()
        .get_entries()
        .iter()
        .enumerate()
    {
        if item.uuid != uuid {
            continue;
        }

        return Some((index, item));
    }

    None
}

fn get_pending_and_error_entries(ui: &AppWindow) -> Vec<UIHistoryEntry> {
    ui.global::<HistorySetting>()
        .get_entries()
        .iter()
        .filter(|item| {
            item.status == TransactionTileStatus::Pending
                || item.status == TransactionTileStatus::Error
        })
        .collect()
}

pub fn init_history(ui: &AppWindow, network: NetworkType) {
    store_history_entries!(ui).set_vec(vec![]);

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        if cfg!(debug_assertions) {
            _ = add_mock_entries_to_db(10).await;
        }

        let entries = get_from_db(network).await;
        _ = slint::invoke_from_event_loop(move || {
            store_history_entries!(ui_handle.unwrap()).set_vec(entries);
        });
    });
}

pub fn init(ui: &AppWindow) {
    let network = if config::developer_mode().enabled {
        NetworkType::from_str(&config::developer_mode().network).unwrap_or(NetworkType::Main)
    } else {
        NetworkType::Main
    };

    init_history(ui, network);

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_add_history(move |entry| {
        let ui = ui_handle.unwrap();
        store_history_entries!(ui).insert(0, entry.clone());
        _add_history(entry.into());
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_update_history_status(move |uuid, status, is_update_db| {
            let ui = ui_handle.unwrap();

            if let Some((index, mut entry)) = get_entry(&ui, &uuid) {
                entry.status = status;
                store_history_entries!(ui).set_row_data(index, entry.clone());

                if is_update_db {
                    _update_entry(entry.into());
                }
            }
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_refresh_all_pending_and_error_history(move || {
            let ui = ui_handle.unwrap();
            for (index, item) in get_pending_and_error_entries(&ui).into_iter().enumerate() {
                if index == 0 {
                    message_info!(ui, tr("正在刷新..."));
                }

                _refresh_pending_and_error_history(&ui, item);
            }
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_remove_history(move |uuid| {
        let ui = ui_handle.unwrap();

        if let Some((index, _)) = get_entry(&ui, &uuid) {
            store_history_entries!(ui).remove(index);
            _remove_entry(uuid);
            message_success!(ui, tr("删除成功"));
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_open_tx_detail(move |network, hash| {
            let ui = ui_handle.unwrap();
            match NetworkType::from_str(&network) {
                Ok(ty) => {
                    let url = ty.tx_detail_url(&hash);
                    ui.global::<Util>()
                        .invoke_open_url("Default".into(), url.into());
                    message_success!(ui, tr("打开成功"));
                }
                Err(e) => message_warn!(ui, format!("{}. {e:?}", tr("打开失败"))),
            }
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_switch_history_network(move |network| {
            let ui = ui_handle.unwrap();
            match NetworkType::from_str(&network) {
                Ok(ty) => {
                    init_history(&ui, ty);
                }
                Err(e) => message_warn!(ui, format!("{}. {e:?}", tr("刷新失败"))),
            }
        });
}

fn _add_history(entry: HistoryEntry) {
    tokio::spawn(async move {
        _ = db::entry::insert(
            HISTORY_TABLE,
            &entry.uuid,
            &serde_json::to_string(&entry).unwrap(),
        )
        .await;
    });
}

fn _update_entry(entry: HistoryEntry) {
    tokio::spawn(async move {
        _ = db::entry::update(
            HISTORY_TABLE,
            &entry.uuid,
            &serde_json::to_string(&entry).unwrap(),
        )
        .await;
    });
}

fn _remove_entry(uuid: SharedString) {
    tokio::spawn(async move {
        _ = db::entry::delete(HISTORY_TABLE, &uuid).await;
    });
}

fn _refresh_pending_and_error_history(ui: &AppWindow, item: UIHistoryEntry) {
    ui.global::<Logic>().invoke_update_history_status(
        item.uuid.clone(),
        TransactionTileStatus::Loading,
        false,
    );

    let rpc_url_ty = RpcUrlType::from_str(&item.network).unwrap_or(RpcUrlType::Main);

    let ui_handle = ui.as_weak();
    match Signature::from_str(&item.hash) {
        Ok(signature) => {
            tokio::spawn(async move {
                let status = match is_signature_confirmed(
                    rpc_url_ty,
                    &signature,
                    Some(transaction::DEFAULT_TIMEOUT_SECS),
                )
                .await
                {
                    Ok(_) => TransactionTileStatus::Success,
                    Err(e) => {
                        log::debug!("{e:?}");
                        TransactionTileStatus::Error
                    }
                };

                let ui = ui_handle.clone();
                _ = slint::invoke_from_event_loop(move || {
                    ui.unwrap()
                        .global::<Logic>()
                        .invoke_update_history_status(item.uuid, status, true);
                });

                async_message_success(ui_handle, tr("刷新完成"));
            });
        }
        Err(e) => message_warn!(ui_handle.unwrap(), e.to_string()),
    }
}
