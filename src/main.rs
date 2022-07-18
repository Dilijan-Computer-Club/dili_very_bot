#![feature(result_flattening)]
#![allow(clippy::match_like_matches_macro)]

use teloxide::{
    prelude::*,
    types::Chat,
    dispatching::{ dialogue, UpdateHandler },
};

mod error;
mod order;
mod tg_msg;
mod db;
mod utils;
mod urgency;
mod markup;
mod ui;

use db::Db;
use crate::error::Error;
use crate::ui::{State, MyDialogue, MyStorage, HandlerResult};

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
async fn collect_data_handler(db: Db, update: Update) -> bool {
    log::info!("Collecting data...");
    let _ = ui::collect_data(db, update).await;
    false
}

async fn handle_callback_query(
    bot: AutoSend<Bot>,
    q: CallbackQuery,
    mut db: Db,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> handle_callback_query");
    db.collect_data_from_cq(q.clone()).await?;
    log::debug!("   query: {q:?}");

    let uid = q.from.id;
    if q.data.is_none() {
        // Fallback to generic callback query handler
        log::info!("  -> Fallback to generic callback query handler");
        return handle_unknown_callback_query(bot, q, db, dialogue).await;
    }
    let item = q.data.as_ref().unwrap();
    let menu_item = ui::main_menu::MainMenuItem::from_id(item);

    if menu_item.is_none() {
        // Fallback to generic callback query handler
        log::info!("  -> Fallback to generic callback query handler");
        return handle_unknown_callback_query(bot, q, db, dialogue).await;
    }
    let menu_item = menu_item.unwrap();

    let msg = &q.message;
    if msg.is_none() {
        log::warn!("CallbackQuery Message is missing when \
trying to handle ShowMyOrders q = {q:?}");
        return Ok(())
    }
    let msg = msg.as_ref().unwrap();

    ui::main_menu::handle_item(
        bot, &q, db, &msg.chat, msg, uid, menu_item, dialogue).await?;
    Ok(())
}

async fn handle_unknown_callback_query(
    bot: AutoSend<Bot>,
    q: CallbackQuery,
    db: Db,
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
    let pcid = db.pub_chat_id_from_cq(q.clone()).await;
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

    dialogue::enter::<Update, MyStorage, State, _>()
        .branch(dptree::filter_async(collect_data_handler))
        .branch(message_handler)
        .branch(callback_query_handler)
        .branch(dptree::case![State::NewOrder(no)]
                .branch(ui::new_order::schema()))
        .branch(dptree::entry())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let db = Db::new();

    let bot = init_bot()?.auto_send();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![MyStorage::new(), db])
        .build() // .setup_ctrlc_handler()
        .dispatch()
        .await;

    Ok(())
}

