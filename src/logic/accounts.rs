use crate::{
    config,
    db::{self, accounts::AccountEntry, ComEntry},
    logic::message::{async_message_success, async_message_warn},
    message_info, message_success, message_warn,
    slint_generatedAppWindow::{AccountEntry as UIAccountEntry, AppWindow, Logic, Store},
    util::{
        self,
        crypto::{self, md5_hex},
        http,
        translator::tr,
    },
};
use anyhow::{Context, Result};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use std::{cmp::Ordering, io::BufReader, time::Duration};
use uuid::Uuid;
use wallet::{mnemonic, prelude::*};

#[macro_export]
macro_rules! store_accounts {
    ($ui:expr) => {
        $ui.global::<Store>()
            .get_accounts()
            .as_any()
            .downcast_ref::<VecModel<UIAccountEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

fn get_account(ui: &AppWindow, uuid: &str) -> Option<(usize, UIAccountEntry)> {
    for (index, account) in ui.global::<Store>().get_accounts().iter().enumerate() {
        if account.uuid != uuid {
            continue;
        }

        return Some((index, account));
    }

    None
}

fn get_account_by_derive_index(
    ui: &AppWindow,
    derive_index: i32,
) -> Option<(usize, UIAccountEntry)> {
    for (index, account) in ui.global::<Store>().get_accounts().iter().enumerate() {
        if account.derive_index != derive_index {
            continue;
        }

        return Some((index, account));
    }

    None
}

fn parse_com_entry(items: Vec<ComEntry>) -> Vec<AccountEntry> {
    let mut list = vec![];
    for item in items.into_iter() {
        if item.uuid == db::accounts::SECRET_UUID {
            continue;
        }

        let entry = match serde_json::from_str::<AccountEntry>(&item.data) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("{:?}", e);
                continue;
            }
        };

        list.push(entry);
    }

    list
}

fn remove_all_account(ui: &AppWindow) {
    let uuids = ui
        .global::<Store>()
        .get_accounts()
        .iter()
        .map(|item| item.uuid)
        .collect::<Vec<_>>();

    for uuid in uuids.into_iter() {
        ui.global::<Logic>().invoke_remove_account(uuid);
    }
}

fn init_accounts(ui: &AppWindow) {
    store_accounts!(ui).set_vec(vec![]);

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        match db::accounts::select_all().await {
            Ok(items) => {
                let accounts = parse_com_entry(items);

                let ui = ui_handle.clone();
                _ = slint::invoke_from_event_loop(move || {
                    init_accounts_store(&ui.unwrap(), accounts);
                });
            }
            Err(e) => log::warn!("{e:?}"),
        }
    });
}

fn init_accounts_store(ui: &AppWindow, accounts: Vec<AccountEntry>) {
    let list = accounts
        .into_iter()
        .map(|item| item.into())
        .collect::<Vec<UIAccountEntry>>();

    store_accounts!(ui).set_vec(list);

    let current_derive_index = 0; // TODO

    match get_account_by_derive_index(&ui, current_derive_index) {
        Some((_, account)) => {
            ui.global::<Store>().set_is_show_setup_page(false);
            ui.global::<Store>().set_current_account(account.into());
        }
        None => {
            if store_accounts!(ui).row_count() > 0 {
                let account = store_accounts!(ui).row_data(0).unwrap();
                ui.global::<Store>().set_current_account(account);
                ui.global::<Store>().set_is_show_setup_page(false);
                // TODO: update secret info
            } else {
                ui.global::<Store>().set_is_show_setup_page(true);
            }
        }
    }
}

pub fn init(ui: &AppWindow) {
    init_accounts(ui);

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_new_mnemonics(move |count| {
        let ui = ui_handle.unwrap();

        match count {
            12 => {
                let mn = mnemonic::generate_mnemonic(MnemonicType::Words12);
                let mn = mnemonic::mnemonic_to_str(&mn)
                    .split(char::is_whitespace)
                    .map(|item| item.to_string().into())
                    .collect::<Vec<_>>();
                VecModel::from_slice(&mn)
            }
            24 => {
                let mn = mnemonic::generate_mnemonic(MnemonicType::Words24);
                let mn = mnemonic::mnemonic_to_str(&mn)
                    .split(char::is_whitespace)
                    .map(|item| item.to_string().into())
                    .collect::<Vec<_>>();
                VecModel::from_slice(&mn)
            }
            _ => {
                message_warn!(ui, format!("{}", tr("生成组记词失败")));
                VecModel::from_slice(&vec![])
            }
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_paste_mnemonics(move || {
        let ui = ui_handle.unwrap();
        let mn = ui.global::<Logic>().invoke_copy_from_clipboard();
        let mns = mn
            .split(char::is_whitespace)
            .map(|item| item.to_string().into())
            .collect::<Vec<_>>();

        if mns.len() != 12 || mns.len() != 24 {
            let empty_mns = (0..12)
                .map(|_| SharedString::new())
                .collect::<Vec<SharedString>>();
            VecModel::from_slice(&empty_mns)
        } else {
            VecModel::from_slice(&mns)
        }
    });

    ui.global::<Logic>()
        .on_join_mnemonics(move |mnemonics| mnemonics.iter().collect::<Vec<_>>().join(" ").into());

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_is_valid_mnemonic(move |mnemonics| {
        let ui = ui_handle.unwrap();
        let mn = ui.global::<Logic>().invoke_join_mnemonics(mnemonics);

        if !wallet::mnemonic::is_valid_mnemonic(&mn, MnemonicType::Words12)
            && !wallet::mnemonic::is_valid_mnemonic(&mn, MnemonicType::Words24)
        {
            message_warn!(ui, tr("组记词数量不对，仅支持12和24个组记词"));
            return false;
        }

        if wallet::mnemonic::mnemonic_from_phrase(&mn).is_err() {
            message_warn!(ui, tr("非法组记词"));
            return false;
        }

        true
    });

    // let ui_handle = ui.as_weak();
    // ui.global::<Logic>().on_cache_mnemonics(move |mnemonics| {
    //     let ui = ui_handle.unwrap();
    //     let mut account = ui.global::<Store>().get_current_account();
    //     account.mnemonic = ui.global::<Logic>().invoke_join_mnemonics(mnemonics);
    //     match _cache_mnemonic(&account.mnemonic) {
    //         Ok(pubkey) => {
    //             account.pubkey = pubkey.into();
    //             ui.global::<Store>().set_current_account(account);
    //         }
    //         Err(e) => message_warn!(ui, format!("{}. {}: {e:?}", tr("出错"), tr("原因"))),
    //     }
    // });

    // let ui_handle = ui.as_weak();
    // ui.global::<Logic>().on_save_password(move |password| {
    //     let ui = ui_handle.unwrap();
    //     let mut account = ui.global::<Store>().get_current_account();

    //     match _save_password(&password, &account.mnemonic) {
    //         Ok(mn) => {
    //             account.mnemonic = mn;
    //             account.uuid = Uuid::new_v4().to_string();
    //             account.name = "Account-1".into();
    //             ui.global::<Store>().set_current_account(account);
    //         }
    //         Err(e) => message_warn!(ui, format!("{}. {}: {e:?}", tr("出错"), tr("原因"))),
    //     }
    // });
}

// fn _cache_mnemonic(mnemonic: &str) -> Result<String> {
//     let passphrase = crypto::hash(mnemonic);
//     let mn = mnemonic::mnemonic_from_phrase(mnemonic)?;
//     let seed = wallet::seed::generate_seed(&mn, &passphrase);

//     let seed_bytes = wallet::seed::derive_seed_bytes(&seed.as_bytes(), 0)?;
//     let keypair = wallet::address::generate_keypair(&seed_bytes)?;
//     Ok(keypair.pubkey().to_string())
// }

// fn _save_password(password: &str, &mn: &str) -> Result<()> {
//     let mn = crypto::encrypt(password, mn)?;
//     let conf = config::all();
//     conf.wallet.password = crypto::hash(password);
//     conf.save()
// }
