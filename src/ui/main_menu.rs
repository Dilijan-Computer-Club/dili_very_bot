use crate::db::Db;
use crate::ui::{self, HandlerResult, MyDialogue};

use teloxide::{
    prelude::*,
    payloads::SendMessageSetters,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup,
        Chat,
    },
};

#[derive(Clone, Copy, Debug)]
pub enum MainMenuItem {
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

/// Shows the main menu with buttons
pub async fn main_menu(
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


pub async fn handle_item(
    bot: AutoSend<Bot>,
    q: &CallbackQuery,
    db: Db,
    chat: &Chat,
    msg: &Message,
    uid: UserId,
    menu_item: MainMenuItem,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("main_menu = {menu_item:?}");
    log::info!("-> main_menu-handle_item -- delete_message");
    bot.delete_message(dialogue.chat_id(), msg.id).await?;
    match menu_item {
        MainMenuItem::NewOrder => {
            dialogue.update(
                ui::State::NewOrder(ui::new_order::State::default())).await?;
            ui::new_order::send_initial_message(
                bot, dialogue.chat_id()).await?;
        },
        MainMenuItem::ShowMyOrders => {
            let pcid = ui::pcid_or_err(&bot, &db, msg, &dialogue).await?;
            ui::show_my_orders(
                bot, db, pcid, chat, q.from.id, dialogue).await?;
        },
        MainMenuItem::ListActiveOrders => {
            let pcid = ui::pcid_or_err(&bot, &db, msg, &dialogue).await?;
            ui::list_active_orders(
                bot, db, pcid, chat, uid, dialogue).await?;
        },
        MainMenuItem::MyAssignments => {
            let pcid = ui::pcid_or_err(&bot, &db, msg, &dialogue).await?;
            ui::list_my_assignments(
                bot, db, pcid, chat, uid, dialogue).await?;
        }
    }
    Ok(())
}
