use super::tr::tr;
use crate::{
    db::{
        self,
        def::{TokenTileEntry, TOKENS_TABLE},
        ComEntry,
    },
    logic::message::{async_message_success, async_message_warn},
    message_success, message_warn,
    slint_generatedAppWindow::{
        AppWindow, Logic, Store, TokenTileEntry as UITokenTileEntry, TokensSetting, Util,
    },
};
use anyhow::{bail, Context, Result};
use cutil::crypto;
use slint::{ComponentHandle, Model, SharedString, VecModel, Weak};
use std::{cmp::Ordering, str::FromStr};
use uuid::Uuid;
use wallet::{
    network::{NetworkType, RpcUrlType},
    prelude::*,
    transaction::{self, DEFAULT_TIMEOUT_SECS},
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

async fn get_from_db() -> Vec<UITokenTileEntry> {
    match db::entry::select_all(TOKENS_TABLE).await {
        Ok(items) => items
            .into_iter()
            .filter_map(|item| serde_json::from_str::<TokenTileEntry>(&item.data).ok())
            .map(|item| item.into())
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

pub fn init_tokens(ui: &AppWindow) {
    store_tokens_setting_entries!(ui).set_vec(vec![]);

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        let entries = get_from_db().await;

        let ui_handle = ui_handle.clone();
        _ = slint::invoke_from_event_loop(move || {
            let ui = ui_handle.unwrap();
            store_tokens_setting_entries!(ui).set_vec(entries);
        });
    });
}

pub fn init(ui: &AppWindow) {
    init_tokens(ui);

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_add_token(move |token| {
        let ui = ui_handle.unwrap();
        store_tokens_setting_entries!(ui).push(token.clone().into());
        _add_token(token.into());
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_remove_token(move |uuid| {
        let ui = ui_handle.unwrap();

        if let Some((index, _)) = get_entry(&ui, &uuid) {
            store_tokens_setting_entries!(ui).remove(index);
            _remove_entry(uuid);
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
