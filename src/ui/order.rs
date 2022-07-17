use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup}
};

use crate::error::Error;
use crate::order::{Order, Action};
use crate::markup;
/// Send a message that shows this order
///
/// Arguments
///
/// uid: UserId for which we show the order actions
///      None if it's a public message
pub async fn send_message(
    order: &Order,
    bot: &mut AutoSend<Bot>,
    uid: Option<UserId>,
    chat_id: ChatId,
    ) -> Result<(), Error> {
    let bot = bot.parse_mode(teloxide::types::ParseMode::Html);

    // TODO escape this
    let description = &order.desc_msg.text;

    let order_id = order.id
        .ok_or("Could not make action for order without id")?;

    let actions = match uid {
        Some(uid) => order.user_actions(uid),
        None      => order.public_actions(),
    };

    let actions: Vec<Action> =
        actions.into_iter()
        .map(|action| Action { kind: action, order_id })
        .collect();
    let buttons = actions_keyboard_markup(&actions);

    let status = order.status();
    let user_link = markup::user_link(&order.from);
    let text = format!("\
{user_link}
{status}

{description}");
    bot.send_message(chat_id, text)
        .reply_markup(buttons).await?;
    Ok(())
}

fn actions_keyboard_markup(actions: &[Action]) -> InlineKeyboardMarkup {
    let btns: Vec<InlineKeyboardButton> = actions
        .iter()
        .map(|a| InlineKeyboardButton::callback(a.human_name(),
                                                a.kbd_button_data()) )
        .collect();
    let rows: Vec<Vec<InlineKeyboardButton>> = btns.chunks(2).map(|c| c.to_vec()).collect();
    InlineKeyboardMarkup::new(rows)
}

