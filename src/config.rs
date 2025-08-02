use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    pub forward_dest_pubkey: String,
    pub fee_payer_pubkey: String,
    pub fee_payer_secret: String,
    pub targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub owner_pubkey: String,
    pub owner_secret: String,
    pub stake_receipt_pubkey: String,
    pub amount: f64, // 送金するBONKの量
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}