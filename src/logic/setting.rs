use super::tr::tr;
use crate::{
    config,
    slint_generatedAppWindow::{AppWindow, Logic, Store, Theme},
    slint_generatedAppWindow::{SettingDeveloperMode, SettingSecurityPrivacy},
};
use slint::ComponentHandle;

pub fn init(ui: &AppWindow) {
    init_setting(&ui);

    ui.global::<Store>()
        .set_is_first_run(config::is_first_run());

    ui.global::<Store>()
        .set_is_show_landing_page(config::is_first_run());

    ui.global::<Logic>()
        .on_tr(move |_is_cn, text| tr(text.as_str()).into());

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_get_setting_ui(move || {
        let ui = ui_handle.unwrap();
        ui.global::<Store>().get_setting_ui()
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_set_setting_ui(move |mut setting| {
        let font_size = u32::min(50, u32::max(10, setting.font_size.parse().unwrap_or(16)));
        setting.font_size = slint::format!("{}", font_size);

        ui_handle
            .unwrap()
            .global::<Store>()
            .set_setting_ui(setting.clone());

        let mut all = config::all();
        all.ui.font_size = font_size.into();
        all.ui.font_family = setting.font_family.into();
        all.ui.language = setting.language.into();
        all.ui.is_dark = setting.is_dark;
        _ = config::save(all);
    });

    ui.global::<Logic>().on_get_current_network(move || {
        let setting = config::developer_mode();
        if setting.enabled {
            setting.network.into()
        } else {
            "main".into()
        }
    });

    ui.global::<Logic>().on_get_setting_developer_mode(move || {
        let setting = config::developer_mode();

        SettingDeveloperMode {
            enabled: setting.enabled,
            network: setting.network.into(),
        }
    });

    ui.global::<Logic>()
        .on_set_setting_developer_mode(move |setting| {
            let mut all = config::all();
            all.developer_mode.enabled = setting.enabled;
            all.developer_mode.network = setting.network.into();
            _ = config::save(all);
        });

    ui.global::<Logic>()
        .on_get_setting_security_privacy(move || {
            let setting = config::security_privacy();

            SettingSecurityPrivacy {
                max_prioritization_fee: slint::format!("{}", setting.max_prioritization_fee),
            }
        });

    ui.global::<Logic>()
        .on_set_setting_security_privacy(move |setting| {
            let mut all = config::all();
            all.security_privacy.max_prioritization_fee = setting
                .max_prioritization_fee
                .parse::<u64>()
                .unwrap_or(1000_u64);
            _ = config::save(all);
        });
}

fn init_setting(ui: &AppWindow) {
    let config = config::ui();
    let mut ui_setting = ui.global::<Store>().get_setting_ui();

    let font_size = u32::min(50, u32::max(10, config.font_size));
    ui_setting.font_size = slint::format!("{}", font_size);
    ui_setting.font_family = config.font_family.into();
    ui_setting.language = config.language.into();
    ui_setting.is_dark = config.is_dark;

    ui.global::<Theme>().invoke_set_dark(config.is_dark);
    ui.global::<Store>().set_setting_ui(ui_setting);
}
