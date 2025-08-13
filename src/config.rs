use std::path::Path;

use log::{debug, warn};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub whitelisted_chats: Vec<i64>,
}

pub fn load_config_or_default<P: AsRef<Path>>(filename: P) -> Config {
    std::fs::read_to_string(filename)
        .inspect_err(|e| {
            warn!("Failed to read config due to error - {e:?} - using default config");
        })
        .map(|input| {
            serde_json::from_str::<Config>(input.as_str())
                .inspect(|cfg| {
                    debug!("Loaded config successfully - {cfg:?}");
                })
                .inspect_err(|e| {
                    warn!(
                        "Failed to deserialize config due to error - {e:?} - using default config"
                    );
                })
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

Modified at 2025-09-30 17:01:31.566360
Additional data: 936956
