use std::sync::OnceLock;

use anyhow::anyhow;
use log::warn;
use regex::Regex;
use rust_decimal::{Decimal, dec};
use serde::Deserialize;

const ONE_THOUSAND: Decimal = Decimal::ONE_THOUSAND;
const ONE_MILLION: Decimal = dec!(1_000_000);
const ONE_BILLION: Decimal = dec!(1_000_000_000);

fn format_human_readable(num: Decimal, decimal_places: usize) -> String {
    let abs_num = num.abs();
    let prec = decimal_places;
    
    if abs_num >= ONE_BILLION {
        format!("{:.prec$}B", num / ONE_BILLION)
    } else if abs_num >= ONE_MILLION {
        format!("{:.prec$}M", num / ONE_MILLION)
    } else if abs_num >= ONE_THOUSAND {
        format!("{:.prec$}K", num / ONE_THOUSAND)
    } else {
        format!("{:.prec$}", num)
    }
}

#[derive(Debug, Deserialize)]
pub struct TokenInfo {
    pub id: String,
    pub name: String,
    pub symbol: String,
    #[serde(default)]
    pub launchpad: Option<String>,
    pub mcap: Option<Decimal>,
}

impl TokenInfo {
    pub fn trenchradar_url(&self) -> String {
        format!("https://trench.bot/bundles/{}", self.id)
    }

    pub fn rugcheck_url(&self) -> String {
        format!("https://rugcheck.xyz/tokens/{}", self.id)
    }

    pub fn gmgn_url(&self) -> String {
        format!("https://gmgn.ai/sol/token/{}", self.id)
    }

    pub fn meteora_pools(&self) -> String {
        format!("https://app.meteora.ag/pools#dlmm?search={}", self.id)
    }

    pub fn human_readable_mcap(&self) -> String {
        match self.mcap {
            Some(mcap) => format_human_readable(mcap, 2),
            None => {
                warn!("Token {} has no mcap", self.id);
                "".to_owned()
            },
        }
    }
}

pub async fn retrieve_token_info(
    token_ca: &str,
    client: reqwest::Client,
) -> anyhow::Result<TokenInfo> {
    let url = format!("https://lite-api.jup.ag/tokens/v2/search?query={token_ca}");

    let mut response = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<TokenInfo>>()
        .await?;

    response.pop().ok_or(anyhow!("Token CA {token_ca} not found on Jupiter"))
}

pub static TOKEN_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn init_token_regex() {
    // this is safe as long as the regex itself is valid
    let regex = Regex::new(
        "(?:https:\\/\\/gmgn\\.ai\\/sol\\/token\\/(?:[a-zA-Z0-9]{4,10}_)?|^| )(?P<token_ca>[1-9A-HJ-NP-Za-km-z]{32,44})",
    )
    .unwrap();
    // This is safe if init_pool_regex is called just once directly in the main fn
    TOKEN_REGEX.set(regex).unwrap();
}
