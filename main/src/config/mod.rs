mod conf;
mod data;

pub use conf::{all, appid, backup_recover, db_path, init, is_first_run, proxy, reset, save, ui};
pub use data::Config;
