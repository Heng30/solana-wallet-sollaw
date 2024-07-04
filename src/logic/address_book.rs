use crate::{
    db::{
        self,
        def::{AddressBookEntry, ADDRESS_BOOK_TABLE},
    },
    message_success, message_warn,
    slint_generatedAppWindow::{
        AddressBookEntry as UIAddressBookEntry, AddressBookSetting, AppWindow, Icons, Logic,
        SettingDetailIndex, Store,
    },
};
use super::tr::tr;
use image::Rgb;
use qrcode::QrCode;
use slint::{ComponentHandle, Image, Model, Rgb8Pixel, SharedPixelBuffer, SharedString, VecModel};
use uuid::Uuid;

#[macro_export]
macro_rules! store_address_book_entries {
    ($ui:expr) => {
        $ui.global::<AddressBookSetting>()
            .get_entries()
            .as_any()
            .downcast_ref::<VecModel<UIAddressBookEntry>>()
            .expect("We know we set a VecModel earlier")
    };
}

async fn get_from_db() -> Vec<UIAddressBookEntry> {
    match db::entry::select_all(ADDRESS_BOOK_TABLE).await {
        Ok(items) => items
            .into_iter()
            .filter_map(|item| serde_json::from_str::<AddressBookEntry>(&item.data).ok())
            .map(|item| item.into())
            .collect(),
        Err(e) => {
            log::warn!("{:?}", e);
            vec![]
        }
    }
}

fn get_entry(ui: &AppWindow, uuid: &str) -> Option<(usize, UIAddressBookEntry)> {
    for (index, address) in ui
        .global::<AddressBookSetting>()
        .get_entries()
        .iter()
        .enumerate()
    {
        if address.uuid != uuid {
            continue;
        }

        return Some((index, address));
    }

    None
}

pub fn init_address_book(ui: &AppWindow) {
    store_address_book_entries!(ui).set_vec(vec![]);

    let ui_handle = ui.as_weak();
    tokio::spawn(async move {
        let entries = get_from_db().await;
        _ = slint::invoke_from_event_loop(move || {
            store_address_book_entries!(ui_handle.unwrap()).set_vec(entries);
        });
    });
}

pub fn init(ui: &AppWindow) {
    init_address_book(ui);

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_qr_code(move |text| {
        let ui = ui_handle.unwrap();
        match QrCode::new(text) {
            Ok(code) => {
                let qrc = code.render::<Rgb<u8>>().build();

                let buffer = SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(
                    qrc.as_raw(),
                    qrc.width(),
                    qrc.height(),
                );
                Image::from_rgb8(buffer)
            }
            _ => ui.global::<Icons>().get_no_data(),
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_add_address(move || {
        let ui = ui_handle.unwrap();
        let address = AddressBookEntry {
            uuid: Uuid::new_v4().to_string(),
            name: String::default(),
            address: String::default(),
        };

        store_address_book_entries!(ui).push(address.clone().into());
        ui.global::<AddressBookSetting>()
            .set_current_entry(address.clone().into());
        ui.global::<Store>()
            .set_current_setting_detail_index(SettingDetailIndex::AddressBookDetail);
        _add_address(address);
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_update_address_name(move |uuid, name| {
            let ui = ui_handle.unwrap();

            match get_entry(&ui, &uuid) {
                Some((index, mut address)) => {
                    address.name = name.into();
                    store_address_book_entries!(ui).set_row_data(index, address.clone());

                    _update_entry(address.into());
                    message_success!(ui, tr("更新地址成功"));
                }
                None => message_warn!(ui, "更新地址失败"),
            }
        });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_update_address(move |uuid, addr| {
        let ui = ui_handle.unwrap();

        match get_entry(&ui, &uuid) {
            Some((index, mut address)) => {
                address.address = addr.into();
                store_address_book_entries!(ui).set_row_data(index, address.clone());

                _update_entry(address.into());
                message_success!(ui, tr("更新地址成功"));
            }
            None => message_warn!(ui, "更新地址失败"),
        }
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_remove_address(move |uuid| {
        let ui = ui_handle.unwrap();

        if let Some((index, _)) = get_entry(&ui, &uuid) {
            store_address_book_entries!(ui).remove(index);
            _remove_entry(uuid);
            ui.global::<Store>()
                .set_current_setting_detail_index(SettingDetailIndex::AddressBook);
            message_success!(ui, tr("删除地址成功"));
        }
    });
}

fn _add_address(address: AddressBookEntry) {
    tokio::spawn(async move {
        _ = db::entry::insert(
            ADDRESS_BOOK_TABLE,
            &address.uuid,
            &serde_json::to_string(&address).unwrap(),
        )
        .await;
    });
}

fn _update_entry(address: AddressBookEntry) {
    tokio::spawn(async move {
        _ = db::entry::update(
            ADDRESS_BOOK_TABLE,
            &address.uuid,
            &serde_json::to_string(&address).unwrap(),
        )
        .await;
    });
}

fn _remove_entry(uuid: SharedString) {
    tokio::spawn(async move {
        _ = db::entry::delete(ADDRESS_BOOK_TABLE, &uuid).await;
    });
}
