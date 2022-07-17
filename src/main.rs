#![feature(result_flattening)]
#![allow(clippy::match_like_matches_macro)]

use teloxide::{
    prelude::*,
    types::{ Chat, User, },
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
    if q.message.is_none() {
        log::warn!("query has no message");
        return Ok(())
    }
    let msg = &q.message;
    let chat: &Chat = &msg.as_ref().unwrap().chat;

    if let Some(msg) = msg {
        db.collect_data_from_msg(msg.clone()).await?;
    }

    let uid = q.from.id;
    if let Some(item) = &q.data {
        let menu_item = ui::main_menu::MainMenuItem::from_id(item);

        if msg.is_none() {
            log::warn!("query callback message is missing when \
trying to handle ShowMyOrders q = {q:?}");
            return Ok(())
        }
        let msg = msg.as_ref().unwrap();


        match menu_item {
            Some(menu_item) =>
                ui::main_menu::handle_item(
                    bot, &q, db, chat, msg, uid, menu_item, dialogue).await?,
            None => {
                // Fallback to generic callback query handler
                log::info!("  -> Fallback to generic callback query handler");
                return handle_unknown_callback_query(
                    bot, q, db, dialogue).await;
            }
        }
    } else {
        // Fallback to generic callback query handler
        log::info!("  -> Fallback to generic callback query handler");
        return handle_unknown_callback_query(bot, q, db, dialogue).await;
    }
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
    if let Some(action) = order::Action::try_parse(&data) {
        log::info!("  got action from callback query {action:?}");
        let pcid = db.pub_chat_id_from_cq(q.clone()).await;
        match pcid {
            Ok(pcid) => {
                let changed = handle_order_action(
                    bot.clone(), uid, pcid, action, db, dialogue).await?;
                if changed {
                    if q.message.is_none() {
                        log::warn!("Message is missing in callback query");
                        return Ok(())
                    }
                    let msg = q.message.unwrap();
                    bot.delete_message(msg.chat.id, msg.id).await?;
                }
            },
            Err(e) => {
                log::warn!("-> handle_unknown_callback_query pcid: {e:?}");
                bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
            }
        }
    }

    Ok(())
}

/// Handles order button clicks
///
/// Returns true if order is changed and we need to delete the old message
/// to avoid confusion
async fn handle_order_action(
    mut bot: AutoSend<Bot>,
    uid: UserId,
    pcid: ChatId,
    action: order::Action,
    mut db: Db,
    dialogue: MyDialogue,
) -> Result<bool, Error> {
    let action_type = action.kind;
    let res = db.perform_action(uid, pcid, action).await;
    if let Err(e) = res {
        // handle error here
        bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
        return Ok(false)
    }

    let (prev_status, order) = res.unwrap();

    let mut changed = false;
    if let Some(new_order) = order {
        let new_status = new_order.status();
        if prev_status != new_status {
            changed = true
        }

        // reporting updates:
        //   Any -> active          -- msg to owner
        //   Any -> Assigned        -- msg to both parties
        //   Any -> MarkAsDelivered -- msg to both parties
        //   Any -> Unassign        -- msg to both parties (if assigned)
        //   Any -> ConfirmDelivery -- msg to both parties
        //   Any -> Delete          -- unreachable
        match new_status {
            order::Status::Unpublished => {
                bot.send_message(
                    dialogue.chat_id(), "The order is unpublished")
                    .await?;
            },
            order::Status::Published => {
                if pcid != dialogue.chat_id() {
                    // it's a different chat, so reply here as well as send
                    // a notification to the main public chat
                    bot.send_message(
                        dialogue.chat_id(), "The order is published").await?;
                }
                // Send notification to public chat
                ui::order::send_message(&new_order, &mut bot, None, pcid).await?;
            },
            order::Status::Assigned => {
                let assignee_uid = new_order.assigned.as_ref().unwrap().1;
                let assignee: Option<User> = db.get_user(assignee_uid).await?;
                if let Some(assignee) = assignee {
                    // Send a private message to the order owner
                    let assignee_link = markup::user_link(&assignee);
                    let msg =
                        format!("Order is assigned to {assignee_link}");

                    let priv_chat_id: ChatId = uid.into();
                    bot.send_message(priv_chat_id, msg).await?;
                    ui::order::send_message(
                        &new_order, &mut bot, Some(uid), priv_chat_id).await?;

                    // Send a public message sayng the order is taken
                    let msg = format!("Order is taken by {}",
                                      markup::user_link(&assignee));
                    (&mut bot)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .send_message(pcid, msg).await?;
                    ui::order::send_message(
                        &new_order, &mut bot, None, pcid).await?;
                } else {
                    // no assignee
                    log::warn!("Couldn't find assignee {assignee_uid}");
                }
            },
            order::Status::MarkedAsDelivered => {
                // TODO notifications
            },
            order::Status::DeliveryConfirmed => {
                // TODO notifications
            },
        }

        // bot.send_message(dialogue.chat_id(),
        //     format!("Changed status to {new_status}")).await?;
        log::info!("{prev_status} + {action_type:?} -> {new_status}    {new_order:?}");
    } else {
        bot.send_message(dialogue.chat_id(), "Deleted the order").await?;
    }

    Ok(changed)
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

