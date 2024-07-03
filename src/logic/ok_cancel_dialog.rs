use crate::slint_generatedAppWindow::{AppWindow, Logic, PasswordSetting};
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
                "remove-all-accounts" => {
                    ui.global::<Logic>().invoke_remove_account(user_data);
                    ui.global::<PasswordSetting>().invoke_set(
                        true,
                        handle_type,
                        "".into(),
                    );
                }
                "remove-address-book-entry" => {
                    ui.global::<Logic>().invoke_remove_address(user_data);
                }
                _ => (),
            }
        });
}
