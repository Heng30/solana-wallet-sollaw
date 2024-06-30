use crate::slint_generatedAppWindow::AppWindow;

mod about;
mod clipboard;
mod message;
mod ok_cancel_dialog;
mod setting;
mod util;
mod accounts;

pub fn init(ui: &AppWindow) {
    util::init(&ui);
    clipboard::init(&ui);
    message::init(&ui);
    ok_cancel_dialog::init(&ui);
    about::init(&ui);
    setting::init(&ui);

    accounts::init(&ui);
}
