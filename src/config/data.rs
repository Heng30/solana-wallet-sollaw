use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub config_path: PathBuf,

    #[serde(skip)]
    pub db_path: PathBuf,

    #[serde(skip)]
    pub cache_dir: PathBuf,

    #[serde(skip)]
    pub is_first_run: bool,

    #[serde(default = "appid_default")]
    pub appid: String,

    pub ui: UI,
    pub network: Network,
}

pub fn appid_default() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UI {
    pub font_size: u32,
    pub font_family: String,
    pub language: String,
    pub is_dark: bool,
}

impl Default for UI {
    fn default() -> Self {
        Self {
            font_size: 16,
            font_family: "Default".to_string(),
            language: "cn".to_string(),
            is_dark: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Network {
    pub ty: String,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            ty: "Main".to_string(),
        }
    }
}
