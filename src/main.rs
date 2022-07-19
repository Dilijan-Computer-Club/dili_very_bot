#![feature(result_flattening)]
#![allow(clippy::match_like_matches_macro)]

use teloxide::{
    prelude::*,
    types::Chat,
    dispatching::{dialogue, UpdateHandler},
    dispatching::dialogue::{ErasedStorage, RedisStorage, Storage},
};

mod error;
mod order;
mod tg_msg;
mod db;
mod utils;
mod urgency;
mod markup;
mod ui;
mod data_gathering;
mod logger;

use db::Db;
use crate::error::Error;
use crate::ui::{State, MyDialogue, HandlerResult, MyStorage};

pub type Offset = chrono::offset::Utc;
pub type DateTime = chrono::DateTime<Offset>;
const REDIS_URL: &str = "redis://127.0.0.1/";


fn init_bot() -> Result<Bot, Error> {
    use std::io::Read;
    let mut file = std::fs::File::open("key")?;
    let mut key = String::new();
    file.read_to_string(&mut key)?;
    Ok(Bot::new(key))
}

/// A filter for that dptree thing that collects data that we might need later
/// it's a filter because I don't know another way to handle all events
/// and passing them to other handlers
async fn collect_data_handler(db: Db, update: Update, state: State) -> bool {
    log::info!("Collecting data... \nstate = {state:?}");
    let _ = ui::collect_data(db, update).await;
    false
}

async fn handle_callback_query(
    bot: AutoSend<Bot>,
    q: CallbackQuery,
    dialogue: MyDialogue,
    mut db: Db,
) -> HandlerResult {
    log::info!("-> handle_callback_query");
    data_gathering::collect_data_from_cq(&mut db, q.clone()).await?;
    log::debug!("   query: {q:?}");

    if q.data.is_none() {
        // Fallback to generic callback query handler
        log::info!("  -> Fallback to generic callback query handler");
        return handle_unknown_callback_query(bot, q, db, dialogue).await;
    }
    let data = q.data.as_ref().unwrap();

    if ! ui::main_menu::try_handle_item(
        bot.clone(), dialogue.clone(), &q, db.clone(), data).await? {

        // Fallback to generic callback query handler
        log::info!("  -> Fallback to generic callback query handler");
        return handle_unknown_callback_query(bot, q, db, dialogue).await;
    }

    Ok(())
}

async fn handle_unknown_callback_query(
    bot: AutoSend<Bot>,
    q: CallbackQuery,
    mut db: Db,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> handle_callback_query
data: {:?}
query: {q:?}", q.data);

    if q.data.is_none() {
        log::info!(" -> handle_callback_query: no data, skipping message");
        return Ok(())
    }
    let data = q.data.clone().unwrap();

    let uid: UserId = q.from.id;
    let action = order::Action::try_parse(&data);
    if action.is_none() {
        return Ok(())
    }
    let action = action.unwrap();

    log::info!("  got action from callback query {action:?}");
    let pcid = data_gathering::pub_chat_id_from_cq(&mut db, q.clone()).await;
    if let Err(e) = pcid {
        log::warn!("-> handle_unknown_callback_query pcid: {e:?}");
        bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
        return Ok(())
    }
    let pcid = pcid.unwrap();

    let changed = ui::order_action::handle_order_action(
        bot.clone(), uid, pcid, action, db, dialogue).await?;

    if changed {
        if q.message.is_none() {
            log::warn!("Message is missing in callback query");
            return Ok(())
        }

        // The order is changed so we better delete the old
        // messege showing the order
        let msg = q.message.unwrap();
        log::info!("deleting old order message {}", msg.id);
        bot.delete_message(msg.chat.id, msg.id).await?;
    }

    Ok(())
}

pub fn schema() -> UpdateHandler<Error> {
    let command_handler = teloxide::filter_command::<ui::commands::Command, _>()
        .endpoint(ui::commands::handle_command);
    let message_handler = Update::filter_message()
        .chain(dptree::entry())
        .branch(command_handler);

    let callback_query_handler =
        Update::filter_callback_query()
            .endpoint(handle_callback_query);

    dialogue::enter::<Update, ErasedStorage<State>, State, _>()
        .branch(dptree::filter_async(collect_data_handler))
        .branch(dptree::case![State::NewOrder(no)]
                .branch(ui::new_order::schema()))
        .branch(message_handler)
        .branch(callback_query_handler)
        .branch(dptree::entry())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    logger::init();
    log::info!("Starting bot...");

    let db = Db::new().await?;

    let bot = init_bot()?.auto_send();

    // let storage = MyStorage::new();
    let storage: MyStorage =
        RedisStorage::open(REDIS_URL, dialogue::serializer::Json)
            .await?.erase();
    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![storage, db])
        .build()//  .setup_ctrlc_handler()
        .dispatch()
        .await;

    Ok(())
}

