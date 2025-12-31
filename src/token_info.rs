use std::sync::OnceLock;

use anyhow::anyhow;
use log::{debug, warn};
use regex::{Regex, RegexBuilder};
use rust_decimal::{Decimal, dec};
use rust_translate::translate_to_english;
use serde::Deserialize;

use crate::APP_CONFIG;

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
struct EvmTokenInfoSerialized {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub market_cap: Decimal,
    pub created_at: Option<String>,
}

#[derive(Debug)]
pub struct EvmTokenInfo {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub mcap: Decimal,
    pub chain: Chain,
}

impl EvmTokenInfo {
    pub fn gmgn_url(&self) -> String {
        let chain = match self.chain {
            Chain::Bsc => "bsc",
            Chain::Base => "base",
        };
        format!("https://gmgn.ai/{chain}/token/{}", self.id)
    }

    pub fn defined_url(&self) -> String {
        let chain = match self.chain {
            Chain::Bsc => "bsc",
            Chain::Base => "base",
            // Chain::Arbitrum => "arb",
            // Chain::Monad => "mon",
        };

        format!("https://www.defined.fi/{chain}/{}", self.id) 
    }

    pub fn dextools_url(&self) -> String {
        let chain = match self.chain {
            Chain::Bsc => "bnb",
            Chain::Base => "base",
        };

        format!("https://www.dextools.io/app/en/{chain}/pair-explorer/{}", self.id)
    }

    pub fn uniswap_add_to_usdt_pool(&self) -> String {
        self.uniswap_add_to_pool(self.get_usdt_ca())
    }

    pub fn uniswap_add_to_usdc_pool(&self) -> String {
        self.uniswap_add_to_pool(self.get_usdc_ca())
    }

    pub fn pancake_add_to_usdt_pool(&self) -> String {
        self.pancake_add_to_pool(self.get_usdt_ca())
    }

    pub fn pancake_add_to_usdc_pool(&self) -> String {
        self.pancake_add_to_pool(self.get_usdc_ca())
    }

    fn pancake_add_to_pool(&self, quote: &str) -> String {
        let chain = match self.chain {
            Chain::Bsc => "bsc",
            Chain::Base => "base",
        };

        let base = &self.id;
        format!(
            "https://pancakeswap.finance/liquidity/select/{chain}/v3/{base}/{quote}?chain={chain}",
        )
    }

    fn uniswap_add_to_pool(&self, quote: &str) -> String {
        let chain = match self.chain {
            Chain::Bsc => "bnb",
            Chain::Base => "base",
        };

        let base = &self.id;
        format!(
            "https://app.uniswap.org/positions/create?currencyA={base}&currencyB={quote}&chain={chain}",
        )
    }

    fn get_usdt_ca(&self) -> &'static str {
        match self.chain {
            Chain::Bsc => "0x55d398326f99059ff775485246999027b3197955",
            Chain::Base => "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2",
            // Chain::Arbitrum => "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9",
            // Chain::Monad => "0xe7cd86e13AC4309349F30B3435a9d337750fC82D",
        }
    }

    fn get_usdc_ca(&self) -> &'static str {
        match self.chain {
            Chain::Bsc => "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",
            Chain::Base => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            // Chain::Arbitrum => "0xaf88d065e77c8cc2239327c5edb3a432268e5831",
            // Chain::Monad => "0x754704bc059f8c67012fed69bc8a327a5aafb603",
        }
    }

    pub fn human_readable_mcap(&self) -> String {
        if self.mcap > Decimal::ZERO {
            format_human_readable(self.mcap, 2)
        } else {
            "??.??K".to_owned()
        }
    }

    pub fn chain_name(&self) -> &str {
        match self.chain {
            Chain::Bsc => "BSC",
            Chain::Base => "BASE",
            // Chain::Arbitrum => "ARB",
            // Chain::Monad => "MON",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SolanaTokenInfo {
    pub id: String,
    pub name: String,
    pub symbol: String,
    #[serde(default)]
    pub launchpad: Option<String>,
    // for non-graduated tokens jupiter skips mcap field
    // in the response
    pub mcap: Option<Decimal>,
}

impl SolanaTokenInfo {
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

    pub fn jup_url(&self) -> String {
        format!("https://jup.ag/tokens/{}", self.id)
    }

    pub fn human_readable_mcap(&self) -> String {
        match self.mcap {
            Some(mcap) if mcap > Decimal::ZERO => format_human_readable(mcap, 2),
            _ => {
                warn!("Token {} has no mcap", self.id);
                "??.??K".to_owned()
            }
        }
    }
}

pub async fn retrieve_solana_token_info(
    token_ca: &str,
    client: reqwest::Client,
) -> anyhow::Result<SolanaTokenInfo> {
    let cfg = APP_CONFIG.get().unwrap();
    let url = format!("https://api.jup.ag/tokens/v2/search?query={token_ca}");

    let mut response = client
        .get(url)
        .header("x-api-key", cfg.jup_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<SolanaTokenInfo>>()
        .await?;

    response.pop().ok_or(anyhow!("Token CA {token_ca} not found on Jupiter"))
}

pub static SOLANA_TOKEN_CA_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn init_solana_token_ca_regex() {
    // this is safe as long as the regex itself is valid
    let regex = RegexBuilder::new(
        "(?:https:\\/\\/gmgn\\.ai\\/sol\\/token\\/(?:[a-zA-Z0-9]{4,10}_)?|https:\\/\\/jup\\.ag\\/tokens\\/|^|\\s)(?P<token_ca>[1-9A-HJ-NP-Za-km-z]{32,44})",
    )
    .multi_line(true)
    .build()
    .unwrap();
    // This is safe if init_pool_regex is called just once directly in the main fn
    SOLANA_TOKEN_CA_REGEX.set(regex).unwrap();
}

pub static EVM_TOKEN_CA_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn init_evm_token_ca_regex() {
    // this is safe as long as the regex itself is valid
    let regex = RegexBuilder::new(
        "(?:https:\\/\\/gmgn\\.ai\\/(?:bsc|base)\\/token\\/(?:[a-zA-Z0-9]{4,10}_)?|^|\\s)(?P<token_ca>0x[a-fA-F0-9]{40})",
    )
    .multi_line(true)
    .build()
    .unwrap();
    // This is safe if init_pool_regex is called just once directly in the main fn
    EVM_TOKEN_CA_REGEX.set(regex).unwrap();
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Bsc,
    Base,
    // Arbitrum,
    // Monad,
}

pub async fn retrieve_evm_token_info(
    token_ca: &str,
    chain: Chain,
    client: reqwest::Client,
) -> anyhow::Result<EvmTokenInfo> {
    let chain_str = match chain {
        Chain::Bsc => "bsc",
        Chain::Base => "base",
        // Chain::Arbitrum => "arbitrum",
        // Chain::Monad => "monad",
    };

    let cfg = APP_CONFIG.get().unwrap();

    let url = "https://deep-index.moralis.io/api/v2.2/erc20/metadata";
    debug!("Going to hit url - {url}");

    let mut response = client
        .get(url)
        .query(&[("chain", chain_str), ("addresses[0]", token_ca)])
        .header("X-API-Key", cfg.moralis_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<EvmTokenInfoSerialized>>()
        .await?;

    let mut response = response
        .pop()
        .ok_or(anyhow!("Token CA {token_ca} not found on Moralis at all"))
        .and_then(|info| {
            if info.created_at.is_none() {
                return Err(anyhow!("Token {token_ca} not found on {chain:?}"));
            }

            Ok(EvmTokenInfo {
                id: info.address,
                name: info.name,
                symbol: info.symbol,
                mcap: info.market_cap,
                chain,
            })
        });

    if let Ok(info) = response.as_mut()
        && is_cjk_only(&info.name)
        && let Ok(translation) = translate_to_english(&info.name).await
    {
        let new_name = format!("{} ({})", info.name, translation);
        info.name = new_name;
    }

    response
}

pub async fn translate_token_name() {

}

fn is_cjk_only(s: &str) -> bool {
    s.chars().all(is_cjk_char)
}

fn is_cjk_char(c: char) -> bool {
    c.is_whitespace()
        || matches!(c as u32,
            // CJK Unified Ideographs
            0x4E00..=0x9FFF |
            // CJK Unified Ideographs Extension A
            0x3400..=0x4DBF |
            // CJK Unified Ideographs Extension B-G
            0x20000..=0x2A6DF |
            0x2A700..=0x2B73F |
            0x2B740..=0x2B81F |
            0x2B820..=0x2CEAF |
            0x2CEB0..=0x2EBEF |
            // CJK Compatibility Ideographs
            0xF900..=0xFAFF |
            0x2F800..=0x2FA1F |
            // Hiragana
            0x3040..=0x309F |
            // Katakana
            0x30A0..=0x30FF |
            // Katakana Phonetic Extensions
            0x31F0..=0x31FF |
            // Hangul Syllables
            0xAC00..=0xD7AF |
            // Hangul Jamo
            0x1100..=0x11FF |
            0x3130..=0x318F |
            0xA960..=0xA97F |
            0xD7B0..=0xD7FF
        )
}
