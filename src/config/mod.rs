mod conf;
mod data;

pub use conf::{all, appid, db_path, init, is_first_run, reset, save, ui};
pub use data::Config;
