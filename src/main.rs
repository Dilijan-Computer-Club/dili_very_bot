#![feature(result_flattening)]
#![allow(clippy::match_like_matches_macro)]

use std::error::Error;
use teloxide::{
    prelude::*,
    payloads::SendMessageSetters,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup,
        Chat, ChatKind, UpdateKind
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
mod order_action;
mod tg_msg;
mod db;
mod urgency;

use db::Db;
pub use order::Order;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

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
    #[command(description = "List active orders")]
    ListactiveOrders,
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

fn init_bot() -> Result<Bot, Box<dyn Error>> {
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

pub fn schema() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(dptree::case![Command::Menu].endpoint(main_menu))
        .branch(dptree::case![Command::Start].endpoint(main_menu))
        .branch(dptree::case![Command::ListactiveOrders])
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

    db.print_stats();

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
async fn main() -> Result<(), Box<dyn Error>> {
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
    ListactiveOrders,
    ShowMyOrders,
    MyAssignments,
    NewOrder,
}

impl MainMenuItem {
    pub const fn human_name(&self) -> &'static str {
        match self {
            MainMenuItem::ListactiveOrders => "List active orders",
            MainMenuItem::ShowMyOrders     => "My Orders",
            MainMenuItem::MyAssignments    => "Orders I'm delivering",
            MainMenuItem::NewOrder         => "New Order",
        }
    }

    pub const fn id(&self) -> &'static str {
        match self {
            MainMenuItem::ListactiveOrders => "list_active_orders",
            MainMenuItem::ShowMyOrders     => "show_my_orders",
            MainMenuItem::MyAssignments    => "my_assignments",
            MainMenuItem::NewOrder         => "new_order",
        }
    }

    pub const fn all_items() -> [Self; 4] {
        [ MainMenuItem::ListactiveOrders,
          MainMenuItem::ShowMyOrders,
          MainMenuItem::MyAssignments,
          MainMenuItem::NewOrder ]
    }

    pub fn from_id(s: &str) -> Option<MainMenuItem> {
        match s {
          "list_active_orders" => Some(MainMenuItem::ListactiveOrders),
          "show_my_orders"     => Some(MainMenuItem::ShowMyOrders),
          "my_assignments"     => Some(MainMenuItem::MyAssignments),
          "new_order"          => Some(MainMenuItem::NewOrder),
          _ => None
        }
    }
}

/// Shows the main menu with buttons
async fn main_menu(
    bot: AutoSend<Bot>,
    msg: Message,
    mut db: Db,
) -> HandlerResult {
    db.collect_data_from_msg(msg.clone()).await?;
    log::info!("-> main_menu");

    let main_menu_items = MainMenuItem::all_items()
        .map(|item| [InlineKeyboardButton::callback(item.human_name(),
                         item.id())]);

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
        for order in orders.iter() {
            order.send_message_for(
                &mut bot,
                uid,
                chat.id,
                matches!(chat.kind, ChatKind::Public(_))
            ).await?;
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
        for order in orders.iter() {
            order.send_message_for(
                &mut bot,
                uid,
                chat.id,
                matches!(chat.kind, ChatKind::Public(_))
            ).await?;
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
        for order in orders.iter() {
            order.send_message_for(
                &mut bot,
                uid,
                chat.id,
                matches!(chat.kind, ChatKind::Public(_))
            ).await?;
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


        match menu_item {
            Some(MainMenuItem::NewOrder) => {
                dialogue.update(
                    State::NewOrder(new_order::State::default())).await?;
                new_order::send_initial_message(
                    bot, dialogue.chat_id()).await?;
            },
            Some(MainMenuItem::ShowMyOrders) => {
                let pcid = db.pub_chat_id_from_msg(msg.clone()).await?;
                if pcid.is_none() {
                    log::warn!("Could not get pcid from msg {:?}", &msg);
                    return Ok(())
                }
                let pcid = pcid.unwrap();
                show_my_orders(bot, db, pcid, chat, q.from.id, dialogue).await?;
            },
            Some(MainMenuItem::ListactiveOrders) => {
                let pcid = db.pub_chat_id_from_msg(msg.clone()).await?;
                if pcid.is_none() {
                    log::warn!("Could not get pcid from msg {:?}", &msg);
                    return Ok(())
                }
                let pcid = pcid.unwrap();
                list_active_orders(bot, db, pcid, chat, uid, dialogue).await?;
            },
            Some(MainMenuItem::MyAssignments) => {
                let pcid = db.pub_chat_id_from_msg(msg.clone()).await?;
                if pcid.is_none() {
                    log::warn!("Could not get pcid from msg {:?}", &msg);
                    return Ok(())
                }
                let pcid = pcid.unwrap();
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

    if let Some(action) = order::SpecificAction::try_parse(&data, q.from.id) {
        let pcid = db.pub_chat_id_from_msg(msg.clone()).await?;
        if let Some(pcid) = pcid {
            return handle_order_action(bot, pcid, action, db, dialogue).await;
        }
    }

    Ok(())
}

async fn handle_order_action(
    bot: AutoSend<Bot>,
    pcid: ChatId,
    action: order::SpecificAction,
    mut db: Db,
    dialogue: MyDialogue,
) -> HandlerResult {
    let action_type = action.action.clone();
    let (prev_status, order) = db.perform_action(pcid, action).await?;

    // TODO
    if let Some(new_order) = order {
        let new_status = new_order.status();
        // status updated

        // reporting updates:
        //   Any -> active       -- msg to owner
        //   Any -> AssignedToMe    -- msg to both parties
        //   Any -> MarkAsDelivered -- msg to both parties
        //   Any -> Unassign        -- msg to both parties (if assigned)
        //   Any -> ConfirmDelivery -- msg to both parties
        //   Any -> Delete          -- unreachable
        bot.send_message(dialogue.chat_id(),
            format!("Changed status to {new_status}")).await?;
        log::info!("{prev_status} + {action_type:?} -> {new_status}    {new_order:?}");
    } else {
        bot.send_message(dialogue.chat_id(), "Deleted the order").await?;
    }

    Ok(())
}
