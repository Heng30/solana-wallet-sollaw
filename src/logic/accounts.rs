use crate::{
    config,
    db::{
        self,
        accounts::{AccountEntry, SecretInfo, SECRET_UUID},
        ComEntry,
    },
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

async fn get_secrect_info() -> Result<SecretInfo> {
    let cm = db::accounts::select(SECRET_UUID)
        .await
        .with_context(|| "Get SecretInfo failed")?;
    serde_json::from_str::<SecretInfo>(&cm.data)
        .with_context(|| format!("Parse Json failed. {cm:?}"))
}

async fn insert_secret_info(mut info: SecretInfo) -> Result<()> {
    info.mnemonic = crypto::encrypt(&info.password, &info.mnemonic.as_bytes())?;
    info.password = crypto::hash(&info.password);

    let info = serde_json::to_string(&info)?;

    _ = db::accounts::delete(SECRET_UUID).await;
    db::accounts::insert(SECRET_UUID, &info)
        .await
        .with_context(|| "insert SecretInfo failed")
}

async fn update_secret_info(mut info: SecretInfo, old_password: Option<String>) -> Result<()> {
    if let Some(old_password) = old_password {
        let mn = crypto::decrypt(&old_password, &info.mnemonic)?;
        info.mnemonic = crypto::encrypt(&info.password, &mn)?;
        info.password = crypto::hash(&info.password);
    }

    let info = serde_json::to_string(&info)?;
    db::accounts::update(SECRET_UUID, &info)
        .await
        .with_context(|| "update SecretInfo failed")
}

fn is_valid_secret_info(info: &SecretInfo) -> bool {
    !info.password.is_empty() && !info.mnemonic.is_empty() && info.current_derive_index >= 0
}

fn accounts_sort_fn(a: &UIAccountEntry, b: &UIAccountEntry) -> Ordering {
    a.derive_index.cmp(&b.derive_index)
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

fn get_unused_derive_index(ui: &AppWindow) -> i32 {
    let mut indexs = ui
        .global::<Store>()
        .get_accounts()
        .iter()
        .map(|item| item.derive_index)
        .collect::<Vec<i32>>();

    indexs.sort();

    for (k, v) in indexs.iter().enumerate() {
        if k as i32 != *v {
            return k as i32;
        }
    }

    indexs.len() as i32
}

fn parse_com_entry(items: Vec<ComEntry>) -> (Option<SecretInfo>, Vec<AccountEntry>) {
    let (mut info, mut list) = (None, vec![]);

    for item in items.into_iter() {
        if item.uuid == SECRET_UUID {
            info = serde_json::from_str::<SecretInfo>(&item.data).ok();
            continue;
        }

        let entry = match serde_json::from_str::<AccountEntry>(&item.data) {
            Ok(v) => v,
            _ => continue,
        };

        list.push(entry);
    }

    (info, list)
}

fn init_accounts(ui: &AppWindow) {
    store_accounts!(ui).set_vec(vec![]);
    ui.global::<Store>().set_is_show_setup_page(true);

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        match db::accounts::select_all().await {
            Ok(items) => {
                let (secret_info, accounts) = parse_com_entry(items);
                if secret_info.is_none() || accounts.is_empty() {
                    _ = db::accounts::delete_all().await;
                    return;
                }

                let secret_info = secret_info.unwrap();
                if !is_valid_secret_info(&secret_info) {
                    return;
                }

                _ = slint::invoke_from_event_loop(move || {
                    init_accounts_in_event_loop(&ui_handle.unwrap(), secret_info, accounts);
                });
            }
            Err(e) => log::warn!("{e:?}"),
        }
    });
}

fn init_accounts_in_event_loop(
    ui: &AppWindow,
    secret_info: SecretInfo,
    accounts: Vec<AccountEntry>,
) {
    let mut list = accounts
        .into_iter()
        .map(|item| item.into())
        .collect::<Vec<UIAccountEntry>>();
    list.sort_by(accounts_sort_fn);

    store_accounts!(ui).set_vec(list);

    match get_account_by_derive_index(&ui, secret_info.current_derive_index) {
        Some((_, account)) => {
            ui.global::<Store>().set_is_show_setup_page(false);
            ui.global::<Store>().set_current_account(account.into());
        }
        None => {
            if store_accounts!(ui).row_count() > 0 {
                let account = store_accounts!(ui).row_data(0).unwrap();
                ui.global::<Store>().set_current_account(account);
                ui.global::<Store>().set_is_show_setup_page(false);
                ui.global::<Logic>().invoke_update_current_derive_index(0);
            }
        }
    }
}

fn get_keypair(password: &str, mnemonic: &str, derive_index: i32) -> Result<Keypair> {
    let mnemonic = crypto::decrypt(password, mnemonic)
        .with_context(|| "Decrypt mnemonic with password failed")?;
    let mnemonic = std::str::from_utf8(&mnemonic).with_context(|| "Mnemonic is not valid utf8")?;

    let passphrase = crypto::hash(mnemonic);
    let mn = mnemonic::mnemonic_from_phrase(mnemonic)?;
    let seed = wallet::seed::generate_seed(&mn, &passphrase);

    let seed_bytes = wallet::seed::derive_seed_bytes(&seed.as_bytes(), derive_index as usize)?;
    wallet::address::generate_keypair(&seed_bytes)
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

        if mns.len() == 12 || mns.len() == 24 {
            VecModel::from_slice(&mns)
        } else {
            let empty_mns = (0..12)
                .map(|_| SharedString::new())
                .collect::<Vec<SharedString>>();
            VecModel::from_slice(&empty_mns)
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

    ui.global::<Logic>().on_is_valid_sign_in_info(
        move |username, password_first, password_second| {
            if username.is_empty() {
                return tr("用户名不能为空").into();
            }

            if password_first != password_second {
                return tr("密码不相同").into();
            }

            if password_first.len() < 8 || password_second.len() < 8 {
                return tr("密码不能小于8位").into();
            }

            SharedString::new()
        },
    );

    ui.global::<Logic>().on_is_valid_password(move |password| {
        if password.len() < 8 {
            return tr("密码不能小于8位").into();
        }

        SharedString::new()
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_save_secret_info(move |username, password, mnemonics| {
            let ui = ui_handle.unwrap();
            let mn = ui.global::<Logic>().invoke_join_mnemonics(mnemonics);

            let info = SecretInfo {
                mnemonic: mn.into(),
                password: password.clone().into(),
                current_derive_index: 0,
            };

            let ui_handle = ui.as_weak();
            tokio::spawn(async move {
                if let Err(e) = insert_secret_info(info).await {
                    async_message_warn(ui_handle, format!("{}. {e:?}", tr("保存失败")));
                } else {
                    _ = slint::invoke_from_event_loop(move || {
                        ui_handle
                            .unwrap()
                            .global::<Logic>()
                            .invoke_new_account(username, password);
                    });
                }
            });
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_update_password(move |old_password, new_password| {
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                match get_secrect_info().await {
                    Ok(mut info) => {
                        info.password = new_password.into();
                        match update_secret_info(info, Some(old_password.into())).await {
                            Err(e) => {
                                async_message_warn(ui_handle, format!("{}. {e:?}", tr("保存失败")))
                            }
                            _ => async_message_success(ui_handle, tr("保存成功")),
                        }
                    }
                    Err(e) => async_message_warn(ui_handle, format!("{}. {e:?}", tr("保存失败"))),
                }
            });
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_update_current_derive_index(move |derive_index| {
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                match get_secrect_info().await {
                    Ok(mut info) => {
                        info.current_derive_index = derive_index;
                        if let Err(e) = update_secret_info(info, None).await {
                            async_message_warn(ui_handle, format!("{}. {e:?}", tr("保存失败")));
                        }
                    }
                    Err(e) => async_message_warn(ui_handle, format!("{}. {e:?}", tr("保存失败"))),
                }
            });
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_new_account(move |name, password| {
        _new_account(&ui_handle.unwrap(), name, password);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_update_account(move |uuid, name| {
        let ui = ui_handle.unwrap();
        match get_account(&ui, &uuid) {
            Some((index, mut account)) => {
                account.name = name;

                if ui.global::<Store>().get_current_account().uuid == uuid {
                    ui.global::<Store>().set_current_account(account.clone());
                }
                store_accounts!(ui).set_row_data(index, account.clone());

                _update_account(account.into());
                message_success!(ui, tr("更新账户成功"));
            }
            None => message_warn!(ui, "更新账户失败. 账户不存在"),
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_remove_account(move |uuid| {
        let ui = ui_handle.unwrap();

        if ui.global::<Store>().get_current_account().uuid == uuid {
            message_warn!(ui, tr("不允许删除当前用户"));
        }

        match get_account(&ui, &uuid) {
            Some((index, account)) => {
                store_accounts!(ui).remove(index);
                _remove_account(uuid);
                message_success!(ui, tr("删除账户成功"));
            }
            None => (),
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_switch_account(move |old_uuid, new_uuid| {
            if old_uuid == new_uuid {
                return;
            }

            let ui = ui_handle.unwrap();
            match get_account(&ui, &new_uuid) {
                Some((_, account)) => {
                    ui.global::<Store>().set_current_account(account);
                    // TODO: fetch the account info from the blockchain
                    message_success!(ui, tr("切换账户成功"));
                }
                None => message_success!(ui, tr("切换账户失败，账户不存在")),
            }
        });
}

fn _new_account(ui: &AppWindow, name: SharedString, password: SharedString) {
    let derive_index = get_unused_derive_index(&ui);
    let ui_handle = ui.as_weak();

    tokio::spawn(async move {
        match get_secrect_info().await {
            Ok(info) => {
                if crypto::hash(&password) != info.password {
                    async_message_warn(ui_handle.clone(), tr("创建用户失败，非法密码"));
                }

                match get_keypair(&password, &info.mnemonic, derive_index) {
                    Ok(kp) => {
                        let account = AccountEntry {
                            uuid: Uuid::new_v4().to_string(),
                            name: if name.is_empty() {
                                format!("Account-{derive_index}")
                            } else {
                                name.into()
                            },
                            pubkey: kp.pubkey().to_string(),
                            derive_index,
                        };

                        let data = serde_json::to_string(&account).unwrap();
                        _ = db::accounts::insert(&account.uuid, &data).await;

                        _ = slint::invoke_from_event_loop(move || {
                            let ui = ui_handle.unwrap();

                            if store_accounts!(ui).row_count() == 0 {
                                ui.global::<Store>()
                                    .set_current_account(account.clone().into());
                                store_accounts!(ui).set_vec(vec![account.into()]);
                            } else {
                                store_accounts!(ui).push(account.into());
                            }

                            ui.global::<Store>().set_is_show_setup_page(false);
                            message_success!(ui, tr("创建用户成功"));
                        });
                    }
                    Err(e) => {
                        async_message_warn(ui_handle, format!("{}. {e:?}", tr("创建用户失败")))
                    }
                }
            }
            Err(e) => async_message_warn(ui_handle, format!("{}. {e:?}", tr("创建用户失败"))),
        }
    });
}

fn _update_account(account: AccountEntry) {
    tokio::spawn(async move {
        _ = db::accounts::update(&account.uuid, &serde_json::to_string(&account).unwrap());
    });
}

fn _remove_account(uuid: SharedString) {
    tokio::spawn(async move {
        _ = db::accounts::delete(&uuid);
    });
}
