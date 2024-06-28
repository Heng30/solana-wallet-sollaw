use crate::config;
use crate::slint_generatedAppWindow::AppWindow;
use serde::{Deserialize, Serialize};

mod about;
mod clipboard;
mod message;
mod ok_cancel_dialog;
mod setting;
mod util;

pub fn init(ui: &AppWindow) {
    util::init(&ui);
    clipboard::init(&ui);
    message::init(&ui);
    ok_cancel_dialog::init(&ui);
    about::init(&ui);
    setting::init(&ui);
}
