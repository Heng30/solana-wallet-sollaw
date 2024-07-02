use crate::slint_generatedAppWindow::{AppWindow, Logic};
use slint::ComponentHandle;

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_handle_ok_cancel_dialog(move |handle_type, user_data| {
            let ui = ui_handle.unwrap();

            match handle_type.as_str() {
                "remove-all-cache" => {
                    ui.global::<Logic>().invoke_remove_all_cache();
                }
                "remove-account" => {
                    ui.global::<Logic>().invoke_remove_account(user_data);
                }
                // "recover-from-remote" => {
                //     let setting = ui.global::<Logic>().invoke_get_setting_backup_recover();
                //     ui.global::<Logic>().invoke_recover_from_remote(setting);
                // }
                _ => (),
            }
        });
}
