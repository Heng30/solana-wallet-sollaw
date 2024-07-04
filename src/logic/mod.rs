use crate::slint_generatedAppWindow::AppWindow;

mod about;
mod accounts;
mod address_book;
mod clipboard;
mod message;
mod ok_cancel_dialog;
mod setting;
mod tr;
mod util;

pub fn init(ui: &AppWindow) {
    util::init(&ui);
    clipboard::init(&ui);
    message::init(&ui);
    ok_cancel_dialog::init(&ui);
    about::init(&ui);
    setting::init(&ui);

    accounts::init(&ui);
    address_book::init(&ui);
}
