use teloxide::{
    prelude::*,
    dispatching::dialogue,
};
use serde::{Serialize, Deserialize};

use std::sync::Arc;
use std::time::Duration;

mod collect_data;
pub use collect_data::collect_data;

pub mod main_menu;
pub mod new_order;
mod show_my_orders;
pub use show_my_orders::show_my_orders;

mod list_my_assignments;
pub use list_my_assignments::list_my_assignments;

mod list_active_orders;
pub use list_active_orders::list_active_orders;

pub mod commands;
pub mod order;
pub mod order_action;
pub mod say_hello;
pub mod help;
pub mod me;


use crate::error::Error;
pub type HandlerResult = Result<(), Error>;
pub type MyDialogue = Dialogue<State, dialogue::ErasedStorage<State>>;
// pub type MyStorage = dialogue::InMemStorage<State>;
// pub type MyStorage = dialogue::RedisStorage<State>;
pub type MyStorage = Arc<dialogue::ErasedStorage<State>>;

use crate::data_gathering;
pub const TEMP_MSG_TIMEOUT_MS: u64 = 60_000;
pub const TEMP_MSG_TIMEOUT: std::time::Duration =
    std::time::Duration::from_millis(TEMP_MSG_TIMEOUT_MS);
pub const TEMP_MSG_FAST_TIMEOUT_MS: u64 = 10_000;
pub const TEMP_MSG_FAST_TIMEOUT: std::time::Duration =
    std::time::Duration::from_millis(TEMP_MSG_FAST_TIMEOUT_MS);

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum State {
    #[default]
    Start,
    NewOrder(new_order::State)
}

pub async fn pcid_or_err(bot: &AutoSend<Bot>, db: &mut crate::Db,
    cq: &CallbackQuery, dialogue: &MyDialogue
) -> Result<ChatId, Error> {
    let pcid = data_gathering::pub_chat_id_from_cq(db, cq.clone()).await;
    match pcid {
        Ok(pcid) => Ok(pcid),
        Err(e) => {
            log::warn!("-> handle_callback_query pcid: {e:?}");
            bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
            Err(format!("{e:?}").into())
        }
    }
}

pub async fn text_msg(
    duration: Option<Duration>,
    bot: AutoSend<Bot>,
    cid: ChatId,
    text: &str,
) -> Result<(), Error> {
    let msg = bot.send_message(cid, text).await?;
    if let Some(duration) = duration {
        tokio::spawn(async move {
            log::debug!("deleting temp msg");
            tokio::time::sleep(duration).await;
            // we don't care if we couldn't delete the message
            let _ = bot.delete_message(cid, msg.id).await;
        });
    }
    Ok(())
}

pub async fn html_msg(
    duration: Option<Duration>,
    bot: AutoSend<Bot>,
    cid: ChatId,
    text: &str,
) -> Result<(), Error> {
    let bot = bot.parse_mode(teloxide::types::ParseMode::Html);
    let msg = bot.send_message(cid, text).await?;
    if let Some(duration) = duration {
        tokio::spawn(async move {
            log::debug!("deleting temp msg");
            tokio::time::sleep(duration).await;
            // we don't care if we couldn't delete the message
            let _ = bot.delete_message(cid, msg.id).await;
        });
    }
    Ok(())
}
