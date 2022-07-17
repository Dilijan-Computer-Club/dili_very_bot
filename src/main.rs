#![feature(result_flattening)]
#![allow(clippy::match_like_matches_macro)]

use teloxide::{
    prelude::*,
    payloads::SendMessageSetters,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup,
        Chat, UpdateKind, User,
    },
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler
    },
    utils::command::BotCommands,
};

mod error;
mod new_order;
mod order;
mod tg_msg;
mod db;
mod utils;
mod urgency;
mod markup;

use db::Db;
use crate::error::Error;
pub use order::Order;

type HandlerResult = Result<(), Error>;

#[derive(BotCommands, Clone)]
#[command(rename = "snake_case",
          description = "These commands are supported:")]
enum Command {
    #[command(description = "Start here")]
    Start,
    #[command(description = "Show main menu")]
    Menu,
    #[command(description = "Show help")]
    Help,
    #[command(description = "Debugging")]
    Debug,
    #[command(description = "Greet the bot to make sure it knows you")]
    Hello,
    #[command(description = "List active orders")]
    ListActiveOrders,
    #[command(description = "Show my orders")]
    ListMyOrders,
    #[command(description = "Make New Order")]
    MakeNewOrder,
}

#[derive(Clone, Default, Debug)]
enum State {
    #[default]
    Start,
    NewOrder(new_order::State)
}

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type MyStorage = InMemStorage<State>;

fn init_bot() -> Result<Bot, Error> {
    use std::io::Read;
    let mut file = std::fs::File::open("key")?;
    let mut key = String::new();
    file.read_to_string(&mut key)?;
    Ok(Bot::new(key))
}

async fn blah(db: Db, update: Update) -> bool {
    let _ = collect_data(db, update).await;
    false
}

pub fn schema() -> UpdateHandler<Error> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::Menu].endpoint(main_menu))
        .branch(dptree::case![Command::Start].endpoint(main_menu))
        .branch(dptree::case![Command::Debug].endpoint(debug_msg))
        .branch(dptree::case![Command::ListActiveOrders])
            .endpoint(list_active_orders);

    let message_handler = Update::filter_message()
        .chain(dptree::entry())
        .branch(command_handler);

    let callback_query_handler =
        Update::filter_callback_query()
            .endpoint(handle_callback_query);

    dialogue::enter::<Update, MyStorage, State, _>()
        .branch(dptree::filter_async(blah))
        .branch(message_handler)
        .branch(callback_query_handler)
        .branch(dptree::case![State::NewOrder(no)]
                .branch(new_order::schema()))
        .branch(dptree::entry())
}

async fn collect_data(
    mut db: Db,
    update: Update,
) -> HandlerResult {
    log::debug!("update: {:?}", update);
    let ret = match update.kind {
        UpdateKind::Message(m) => collect_data_from_msg(db.clone(), m).await,
        UpdateKind::EditedMessage(m) => collect_data_from_msg(db.clone(), m).await,
        UpdateKind::ChannelPost(m) => collect_data_from_msg(db.clone(), m).await,
        UpdateKind::EditedChannelPost(m) => collect_data_from_msg(db.clone(), m).await,
        UpdateKind::InlineQuery(q) => db.update_user(q.from).await,
        UpdateKind::ChosenInlineResult(cir) => db.update_user(cir.from).await,
        UpdateKind::CallbackQuery(cq) => {
            if let Some(msg) = cq.message {
                collect_data_from_msg(db.clone(), msg).await
            } else {
                Ok(())
            }
        },
        UpdateKind::ShippingQuery(sq) => db.update_user(sq.from).await,
        UpdateKind::PreCheckoutQuery(pcq) => db.update_user(pcq.from).await,
        UpdateKind::Poll(_poll) => Ok(()),
        UpdateKind::PollAnswer(pa) => db.update_user(pa.user).await,
        UpdateKind::MyChatMember(cmu) => {
            db.update_chat(cmu.chat, Some(cmu.from.clone())).await?;
            db.update_user(cmu.from).await?;
            db.update_user(cmu.new_chat_member.user).await?;
            Ok(())
        }
        UpdateKind::ChatMember(cmu) => {
            db.update_chat(cmu.chat, Some(cmu.from.clone())).await?;
            db.update_user(cmu.from).await?;
            db.update_user(cmu.new_chat_member.user).await?;
            Ok(())
        }
        UpdateKind::ChatJoinRequest(cjr) => {
            db.update_chat(cjr.chat, Some(cjr.from.clone())).await?;
            db.update_user(cjr.from).await?;
            Ok(())
        }
        UpdateKind::Error(_) => Ok(()),
    };

    db.debug_stats().await?;

    ret
}

async fn collect_data_from_msg(
    mut db: Db,
    msg: Message,
) -> HandlerResult {
    db.collect_data_from_msg(msg).await?;
    Ok(())
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

#[derive(Clone, Copy, Debug)]
enum MainMenuItem {
    ListActiveOrders,
    ShowMyOrders,
    MyAssignments,
    NewOrder,
}

impl MainMenuItem {
    pub const fn human_name(&self) -> &'static str {
        match self {
            MainMenuItem::ListActiveOrders => "List active orders",
            MainMenuItem::ShowMyOrders     => "My Orders",
            MainMenuItem::MyAssignments    => "Orders I'm delivering",
            MainMenuItem::NewOrder         => "New Order",
        }
    }

    pub const fn id(&self) -> &'static str {
        match self {
            MainMenuItem::ListActiveOrders => "list_active_orders",
            MainMenuItem::ShowMyOrders     => "show_my_orders",
            MainMenuItem::MyAssignments    => "my_assignments",
            MainMenuItem::NewOrder         => "new_order",
        }
    }

    pub const fn private_items() -> &'static [Self] {
        &[ MainMenuItem::ListActiveOrders,
           MainMenuItem::ShowMyOrders,
           MainMenuItem::MyAssignments,
           MainMenuItem::NewOrder ]
    }

    pub const fn public_items() -> &'static [Self] {
        &[ MainMenuItem::ListActiveOrders ]
    }

    pub fn from_id(s: &str) -> Option<MainMenuItem> {
        match s {
          "list_active_orders" => Some(MainMenuItem::ListActiveOrders),
          "show_my_orders"     => Some(MainMenuItem::ShowMyOrders),
          "my_assignments"     => Some(MainMenuItem::MyAssignments),
          "new_order"          => Some(MainMenuItem::NewOrder),
          _ => None
        }
    }
}

/// Show some debugging info
///
/// TODO limit the displayed information to what's allowed
async fn debug_msg(
    bot: AutoSend<Bot>,
    msg: Message,
    db: Db,
) -> HandlerResult {
    let s = db.debug_stats().await?;
    bot.send_message(msg.chat.id, s).await?;
    Ok(())
}

/// Shows the main menu with buttons
async fn main_menu(
    bot: AutoSend<Bot>,
    msg: Message,
    mut db: Db,
) -> HandlerResult {
    db.collect_data_from_msg(msg.clone()).await?;

    let main_menu_items = if msg.chat.is_private() {
        log::info!("-> main_menu private");
        MainMenuItem::private_items()
    } else {
        log::info!("-> main_menu public");
        MainMenuItem::public_items()
    };
    let main_menu_items = main_menu_items
        .iter()
        .map(|item| [InlineKeyboardButton::callback(
                        item.human_name(), item.id())]);

    bot.send_message(msg.chat.id, "Choose your destiny")
        .reply_markup(InlineKeyboardMarkup::new(main_menu_items)).
        await?;

    Ok(())
}

async fn list_active_orders(
    mut bot: AutoSend<Bot>,
    db: Db,
    pcid: ChatId,
    chat: &Chat,
    uid: UserId,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> list_active_orders");
    let orders = db.orders_by_status(pcid, order::Status::Published).await?;
    if orders.is_empty() {
        bot.send_message(dialogue.chat_id(), "No orders")
            .await?;
    } else {
        bot.send_message(dialogue.chat_id(), "All active orders:").await?;
        let uid = match chat.is_private() {
            true =>  Some(uid),
            false => None,
        };
        for order in orders.iter() {
            order.send_message_for(&mut bot, uid, chat.id).await?;
        }
    }
    Ok(())
}

async fn list_my_assignments(
    mut bot: AutoSend<Bot>,
    db: Db,
    pcid: ChatId,
    chat: &Chat,
    uid: UserId,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> list_my_assignments");
    let orders = db.active_assignments_to(pcid, uid).await?;
    if orders.is_empty() {
        bot.send_message(dialogue.chat_id(), "No assigned orders")
            .await?;
    } else {
        bot.send_message(dialogue.chat_id(), "Orders assigned to you:").await?;
        let uid = match chat.is_private() {
            true => Some(uid),
            false => None,
        };
        for order in orders.iter() {
            order.send_message_for(&mut bot, uid, chat.id).await?;
        }
    }
    Ok(())
}


async fn show_my_orders(
    mut bot: AutoSend<Bot>,
    db: Db,
    pcid: ChatId,
    chat: &Chat,
    uid: UserId,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> show_my_orders");
    let orders = db.orders_submitted_by_user(pcid, uid).await?;
    if orders.is_empty() {
        bot.send_message(dialogue.chat_id(), "You have no current orders")
            .await?;
    } else {
        bot.send_message(dialogue.chat_id(), "Your orders:").await?;
        let uid = match chat.is_private() {
            true => Some(uid),
            false => None,
        };
        for order in orders.iter() {
            order.send_message_for(&mut bot, uid, chat.id).await?;
        }
    }
    dialogue.update(State::Start).await?;
    Ok(())
}

async fn handle_callback_query(
    bot: AutoSend<Bot>,
    q: CallbackQuery,
    mut db: Db,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> handle_callback_query");
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
        let menu_item = MainMenuItem::from_id(item);
        log::info!("main_menu = {menu_item:?}");

        if menu_item.is_some() {
            if let Some(msg) = msg {
                log::info!("-> handle_callback_query -- delete_message");
                bot.delete_message(dialogue.chat_id(), msg.id).await?;
            }
        }

        if msg.is_none() {
            log::warn!("query callback message is missing when \
trying to handle ShowMyOrders q = {q:?}");
            return Ok(())
        }
        let msg = msg.as_ref().unwrap();

        async fn pcid_or_err(bot: &AutoSend<Bot>, db: &Db,
                             msg: &Message, dialogue: &MyDialogue
        ) -> Result<ChatId, Error> {
            let pcid = db.pub_chat_id_from_msg(msg.clone()).await;
            match pcid {
                Ok(pcid) => Ok(pcid),
                Err(e) => {
                    log::warn!("-> handle_callback_query pcid: {e:?}");
                    bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
                    Err(format!("{e:?}").into())
                }
            }
        }

        match menu_item {
            Some(MainMenuItem::NewOrder) => {
                dialogue.update(
                    State::NewOrder(new_order::State::default())).await?;
                new_order::send_initial_message(
                    bot, dialogue.chat_id()).await?;
            },
            Some(MainMenuItem::ShowMyOrders) => {
                let pcid = pcid_or_err(&bot, &db, msg, &dialogue).await?;
                show_my_orders(bot, db, pcid, chat, q.from.id, dialogue).await?;
            },
            Some(MainMenuItem::ListActiveOrders) => {
                let pcid = pcid_or_err(&bot, &db, msg, &dialogue).await?;
                list_active_orders(bot, db, pcid, chat, uid, dialogue).await?;
            },
            Some(MainMenuItem::MyAssignments) => {
                let pcid = pcid_or_err(&bot, &db, msg, &dialogue).await?;
                list_my_assignments(bot, db, pcid, chat, uid, dialogue).await?;
            }
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
    let data = q.data.unwrap();

    if q.message.is_none() {
        log::warn!("Message is missing in callback query");
        return Ok(())
    }
    let msg = q.message.unwrap();

    let uid: UserId = q.from.id;
    if let Some(action) = order::Action::try_parse(&data) {
        log::info!("  got action from callback query {action:?}");
        let pcid = db.pub_chat_id_from_msg(msg.clone()).await;
        match pcid {
            Ok(pcid) => {
                let changed = handle_order_action(
                    bot.clone(), uid, pcid, action, db, dialogue).await?;
                if changed {
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
        // status updated

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
                new_order.send_message_for(&mut bot, None, pcid).await?;
            },
            order::Status::Assigned => {
                let assignee_uid = new_order.assigned.as_ref().unwrap().1;
                let assignee: Option<User> = db.get_user(assignee_uid).await?;
                if let Some(assignee) = assignee {
                    // Send a private message to the order owner
                    if let Some(priv_chat_id) = db.get_priv_chat_id(uid).await? {
                        let assignee_link = markup::user_link(&assignee);
                        let msg =
                            format!("Order is assigned to {assignee_link}");
                        bot.send_message(priv_chat_id, msg).await?;
                        new_order.send_message_for(
                            &mut bot, Some(uid), priv_chat_id).await?;
                    }

                    // Send a public message sayng the order is taken
                    let msg = format!("Order is taken by {}",
                                      markup::user_link(&assignee));
                    (&mut bot)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .send_message(pcid, msg).await?;
                    new_order.send_message_for(
                        &mut bot, None, pcid).await?;
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
