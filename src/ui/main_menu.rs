use crate::db::Db;
use crate::ui::{self, HandlerResult, MyDialogue};
use crate::error::Error;
use std::time::Duration;
use teloxide::{
    prelude::*,
    payloads::SendMessageSetters,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup,
        Chat,
    },
};

const TEMP_MENU_LINK_TIMEOUT: Duration = ui::TEMP_MSG_TIMEOUT;

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
            MainMenuItem::NewOrder         => "New Order 🤘",
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
        &[ MainMenuItem::ListActiveOrders,
           MainMenuItem::NewOrder, ]
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
    pub fn kbd_button(&self) -> InlineKeyboardButton {
        InlineKeyboardButton::callback(self.human_name(), self.id())
    }
}

/// Shows the main menu with buttons
pub async fn main_menu(
    bot: AutoSend<Bot>,
    cid: ChatId,
) -> HandlerResult {
    let main_menu_items = if cid.is_user() {
        log::info!("-> main_menu private");
        MainMenuItem::private_items()
    } else {
        log::info!("-> main_menu public");
        MainMenuItem::public_items()
    };
    let items = main_menu_items.iter().map(|item| item.kbd_button());

    bot.send_message(cid, "Choose your destiny")
        .reply_markup(inline_rows_kbd(items))
        .await?;

    Ok(())
}

/// Give me buttons and I'll give you them aligned as rows in markup
fn inline_rows_kbd<I: Iterator<Item=InlineKeyboardButton>>(btns: I) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(btns.map(|i| [i]))
}

/// Returns true if it was handled, false if it wasn't a menu item
pub async fn try_handle_item(
    bot: AutoSend<Bot>,
    dialogue: MyDialogue,
    q: &CallbackQuery,
    db: Db,
    data: &str,
) -> Result<bool, Error> {
    let menu_item = ui::main_menu::MainMenuItem::from_id(data);
    if menu_item.is_none() {
        return Ok(false)
    }
    let menu_item = menu_item.unwrap();

    let msg = &q.message;
    if msg.is_none() {
        log::warn!("CallbackQuery Message is missing when \
trying to handle ShowMyOrders q = {q:?}");
        return Ok(false)
    }
    let msg = msg.as_ref().unwrap();
    let uid = q.from.id;
    handle_item(bot, q, db, &msg.chat, uid, menu_item, dialogue).await?;
    Ok(true)
}

pub async fn send_menu_link(
    bot: AutoSend<Bot>,
    cid: ChatId,
) -> HandlerResult {
    log::debug!("send_menu_link");
    let msg: Message = bot.send_message(cid, "Open menu like this: /menu").await?;
    let msg_id = msg.id;
    tokio::spawn(async move {
        log::debug!("send_menu_link deleting the link");
        tokio::time::sleep(TEMP_MENU_LINK_TIMEOUT).await;
        let _ = bot.delete_message(cid, msg_id).await;
    });
    Ok(())
}

pub async fn handle_item(
    bot: AutoSend<Bot>,
    q: &CallbackQuery,
    mut db: Db,
    chat: &Chat,
    uid: UserId,
    menu_item: MainMenuItem,
    dialogue: MyDialogue
) -> HandlerResult {
    let cid = dialogue.chat_id();
    log::info!("main_menu = {menu_item:?}");
    if let Some(msg) = &q.message {
        bot.delete_message(cid, msg.id).await?;
    }
    match menu_item {
        MainMenuItem::NewOrder => {
            ui::new_order::start(bot, dialogue, cid, uid).await?
        },
        MainMenuItem::ShowMyOrders => {
            let pcid = ui::pcid_or_err(&bot, &mut db, q, &dialogue).await?;
            ui::show_my_orders(
                bot.clone(), db, pcid, chat, q.from.id, dialogue).await?;
            send_menu_link(bot, cid).await?;
        },
        MainMenuItem::ListActiveOrders => {
            let pcid = ui::pcid_or_err(&bot, &mut db, q, &dialogue).await?;
            ui::list_active_orders(
                bot.clone(), db, pcid, chat, uid, dialogue).await?;
            send_menu_link(bot, cid).await?;
        },
        MainMenuItem::MyAssignments => {
            let pcid = ui::pcid_or_err(&bot, &mut db, q, &dialogue).await?;
            ui::list_my_assignments(
                bot.clone(), db, pcid, chat, uid, dialogue).await?;
            send_menu_link(bot, cid).await?;
        }
    }
    Ok(())
}

