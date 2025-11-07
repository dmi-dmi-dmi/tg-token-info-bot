pub mod config;
pub mod token_info;

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use chrono::{DateTime, Duration, Utc};
use flexi_logger::{AdaptiveFormat, Logger};
use log::{debug, info, warn};
use teloxide::Bot;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Dispatcher, Requester, ResponseResult};
use teloxide::sugar::request::{RequestLinkPreviewExt, RequestReplyExt};
use teloxide::types::{Chat, ChatId, Message, ParseMode, ThreadId, Update, User};
use teloxide::utils::markdown::escape;
use tokio::sync::RwLock;

use crate::config::{Config, load_config_or_default};
use crate::token_info::{init_evm_token_ca_regex, init_solana_token_ca_regex, retrieve_evm_token_info, retrieve_solana_token_info, Chain, EVM_TOKEN_CA_REGEX, SOLANA_TOKEN_CA_REGEX};

static APP_CONFIG: OnceLock<Config> = OnceLock::new();

const ALLOWED_THROTTLING: Duration = Duration::minutes(5);

const AGE_THRESHOLD: Duration = Duration::minutes(6);

type ThrottlingInfo = HashMap<(Cow<'static, str>, ChatId, Option<ThreadId>), DateTime<Utc>>;

type Cache = Arc<RwLock<HashMap<(Cow<'static, str>, ChatId, Option<ThreadId>), DateTime<Utc>>>>;

fn is_whitelisted_chat(chat: &Chat, cfg: &Config) -> bool {
    let ChatId(id) = chat.id;

    cfg.whitelisted_chats.contains(&id)
}

fn is_message_too_old(msg: &Message) -> bool {
    let diff = Utc::now() - msg.date;

    diff > AGE_THRESHOLD
}

async fn message_handler(
    bot: Bot,
    message: Message,
    client: reqwest::Client,
    cache: Arc<RwLock<ThrottlingInfo>>,
) -> ResponseResult<()> {
    debug!("Got {message:?}");

    if is_message_too_old(&message) {
        debug!("Message is too old - skipping it");

        return Ok(());
    }

    if !is_whitelisted_chat(&message.chat, APP_CONFIG.get().unwrap()) {
        debug!("Skipping message since it is not coming from whitelisted chat");
        return Ok(());
    }

    // skip our own messages or messages from other bots
    if let Some(User { is_bot: true, .. }) = message.from {
        debug!("This message is from a bot - ignoring it!");
        return Ok(());
    }

    let maybe_text = message.text().or_else(|| message.caption());
    let Some(msg_text) = maybe_text else {
        warn!("Impossible case - text message doesn't contain text!");
        return Ok(());
    };

    process_solana_cas(&bot, &message, client.clone(), &cache, msg_text).await;
    process_evm_cas(&bot, &message, client, &cache, msg_text).await;

    Ok(())
}

async fn process_evm_cas(
    bot: &Bot,
    message: &Message,
    client: reqwest::Client,
    cache: &Cache,
    msg_text: &str,
) {
    for (_, [token_ca]) in EVM_TOKEN_CA_REGEX
        .get()
        .unwrap()
        .captures_iter(msg_text)
        .map(|c| c.extract())
    {
        info!(
            "FOUND EVM TOKEN CA in the message {:?} - {token_ca}",
            message.id
        );

        if should_we_throttle_ca(message, cache, token_ca).await {
            continue;
        }

        let mut result = None;

        for chain in [Chain::Bsc, Chain::Base] {
            match retrieve_evm_token_info(token_ca, chain, client.clone()).await {
                Ok(data) => {
                    result = Some(data);
                    break;
                },
                Err(err) => {
                    warn!("Failed to retrieve token info {token_ca} on {chain:?} - {err:?}");
                }
            }
        }

        let Some(token_info) = result else {
            continue;
        };

        let message_text = format!(
            "🏷️ *{}* \\- {}\n\
            📜 `{}`\n\
            💵 {} \\- {}\n\
            🦎 [GMGN]({})\n\
            🥞 [P\\. USDT pools]({})     🥞 [P\\. USDC pools]({})\n\
            🦄 [U\\. USDT pools]({})    🦄 [U\\. USDC pools]({})",
            escape(&token_info.symbol),
            escape(&token_info.name),
            token_info.id,
            escape(&token_info.human_readable_mcap()),
            escape(token_info.chain_name()),
            escape(&token_info.gmgn_url()),
            escape(&token_info.pancake_add_to_usdt_pool()),
            escape(&token_info.pancake_add_to_usdc_pool()),
                escape(&token_info.uniswap_add_to_usdt_pool()),
            escape(&token_info.uniswap_add_to_usdc_pool()),
        );

        debug!("Prepared message {message_text}");

        send_reply(bot, message, cache, token_ca, message_text).await;
    }
}

async fn process_solana_cas(
    bot: &Bot,
    message: &Message,
    client: reqwest::Client,
    cache: &Cache,
    msg_text: &str,
) {
    for (_, [token_ca]) in SOLANA_TOKEN_CA_REGEX
        .get()
        .unwrap()
        .captures_iter(msg_text)
        .map(|c| c.extract())
    {
        info!(
            "FOUND SOLANA TOKEN CA in the message {:?} - {token_ca}",
            message.id
        );

        if should_we_throttle_ca(message, cache, token_ca).await {
            continue;
        }

        let data = match retrieve_solana_token_info(token_ca, client.clone()).await {
            Ok(data) => data,
            Err(err) => {
                warn!("Failed to retrieve token info {token_ca} - {err:?}");
                continue;
            }
        };

        let cap = if data.mcap.is_some() {
            format!("💵 {}\n", escape(&data.human_readable_mcap()))
        } else {
            "".to_owned()
        };

        let message_text = format!(
            "🏷️ *{}* \\- {}\n\
            📜 `{}`\n\
            {}\
            🦎 [GMGN]({})            ☄️ [Meteora pools]({})\n\
            🦝 [Rugcheck]({})        📡 [TrenchRadar]({})",
            escape(&data.symbol),
            escape(&data.name),
            data.id,
            cap,
            escape(&data.gmgn_url()),
            escape(&data.meteora_pools()),
            escape(&data.rugcheck_url()),
            escape(&data.trenchradar_url()),
        );

        debug!("Prepared message {message_text}");

        send_reply(bot, message, cache, token_ca, message_text).await;
    }
}

async fn should_we_throttle_ca(message: &Message, cache: &Cache, token_ca: &str) -> bool {
    let value = {
        let cache_guard = cache.read().await;

        let key = (Cow::Borrowed(token_ca), message.chat.id, message.thread_id);
        cache_guard.get(&key).cloned()
    };

    if let Some(latest_mention) = value {
        let now = Utc::now();
        if (now - latest_mention) < ALLOWED_THROTTLING {
            info!(
                "We've sent info on this token {token_ca} not so long time ago so skipping this request for now"
            );
            return true;
        }
    }

    false
}

async fn send_reply(
    bot: &Bot,
    message: &Message,
    cache: &Cache,
    token_ca: &str,
    message_text: String,
) {
    let reply_result = bot
        .send_message(message.chat.id, message_text)
        .parse_mode(ParseMode::MarkdownV2)
        .disable_link_preview(true)
        .disable_notification(true)
        .reply_to(message.id)
        .await;

    match reply_result {
        Ok(msg) => {
            debug!("Sent reply with token info {token_ca} as {}", msg.id);
            {
                let mut cache_guard = cache.write().await;

                let now = Utc::now();
                cache_guard.insert(
                    (
                        Cow::Owned(token_ca.to_owned()),
                        message.chat.id,
                        message.thread_id,
                    ),
                    now,
                );
                debug!("Inserted info about sent token {token_ca} into throttle data");
            }
        }
        Err(e) => {
            warn!("Failed to send token info {token_ca} - {e:?}");
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::from_filename(".envrc").ok();

    Logger::try_with_env_or_str("info")
        .unwrap()
        .adaptive_format_for_stdout(AdaptiveFormat::Opt)
        .log_to_stdout()
        .start()
        .unwrap();

    let Ok(bot_token) = std::env::var("BOT_TOKEN") else {
        panic!("Bot token not found nor in the env variables or in the .env file");
    };

    let config = load_config_or_default("./config.json");
    APP_CONFIG.set(config).unwrap();

    let reqwest_client = reqwest::Client::new();
    init_solana_token_ca_regex();
    init_evm_token_ca_regex();

    let bot = Bot::new(bot_token);

    let throttle_info: Arc<RwLock<ThrottlingInfo>> = Arc::new(RwLock::new(HashMap::new()));

    let handler = Update::filter_message()
        .map(move || reqwest_client.clone())
        .map(move || throttle_info.clone())
        .endpoint(message_handler);

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
