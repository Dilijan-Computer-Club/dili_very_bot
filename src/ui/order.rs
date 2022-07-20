use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId}
};

use crate::error::Error;
use crate::order::{Order, Action, Status};
use crate::markup::{self, time_ago};
use crate::Db;

fn format_status(order: &Order) -> String {
    match order.status() {
        Status::Unpublished => "Not published".to_string(),
        Status::Published =>
            format!("Published {}", time_ago(order.published_at.unwrap())),
        Status::Assigned => {
            let (when, _id, who) = order.assigned.as_ref().unwrap();
            let when = time_ago(*when);
            let to_whom = if let Some(user) = who {
                format!(" to {} ", markup::user_link(user))
            } else {
                "".to_string()
            };

            format!("Assigned {to_whom}{when}")
        },
        Status::MarkedAsDelivered => {
            let (_uid, _u, when) = order.delivered.as_ref().unwrap();
            format!("Marked as deliered {}", time_ago(*when))
        },
        Status::DeliveryConfirmed => {
            let when = order.delivery_confirmed_at.unwrap();
            format!("Delivered {}", time_ago(when))
        }
    }
}

fn format_name(order: &Order) -> String {
    let name = markup::escape_html(&order.name).to_string();
    markup::bold(name)
}

fn format_description(order: &Order) -> String {
    markup::escape_html(&order.description_text).to_string()
}

fn format(order: &Order) -> String {
    let name        = format_name(order);
    let description = format_description(order);
    let status      = format_status(order);
    let user_link   = markup::user_link(&order.customer);
    let price = markup::format_amd(order.price_in_drams);

    let markup = if order.markup_in_drams > 0 {
        format!("\nReward: {}",
                markup::format_amd(order.markup_in_drams))
    } else {
        "".to_string()
    };

    let text = format!("\
{name}
By {user_link}
{status}

{description}

Item cost: {price}{markup}
");
    text
}

/// Send a message that shows this order
///
/// Arguments
///
/// uid: UserId for which we show the order actions
///      None if it's a public message
///
/// prefix: Prepend the order itself with this text
///         Note that it is rendered as HTML
pub async fn send_message<S: AsRef<str>>(
    mut db: Db,
    order: &Order,
    bot: AutoSend<Bot>,
    for_uid: Option<UserId>,
    to_chat_id: ChatId,
    prefix: Option<S>,
) -> Result<(), Error> {
    let mut text = format(order);
    if let Some(prefix) = prefix {
        let prefix = prefix.as_ref();
        text = format!("{prefix}\n\n{text}");
    }

    let order_id = order.id
        .ok_or("Could not make action for order without id")?;

    let actions = match for_uid {
        Some(uid) => order.user_actions(uid),
        None      => order.public_actions(),
    };

    let actions: Vec<Action> =
        actions.into_iter()
        .map(|action| Action { kind: action, order_id })
        .collect();
    let buttons = actions_keyboard_markup(&actions);
    let bot = bot.parse_mode(teloxide::types::ParseMode::Html);
    let msg: Message = bot.send_message(to_chat_id, text)
        .reply_markup(buttons).await?;
    let msg_id = MessageId { message_id: msg.id };
    db.add_msg_id(order_id, to_chat_id, msg_id).await?;
    Ok(())
}

fn actions_keyboard_markup(actions: &[Action]) -> InlineKeyboardMarkup {
    let btns: Vec<InlineKeyboardButton> = actions
        .iter()
        .map(|a| InlineKeyboardButton::callback(a.human_name(),
                                                a.kbd_button_data()) )
        .collect();
    let rows: Vec<Vec<InlineKeyboardButton>> =
        btns.chunks(2).map(|c| c.to_vec()).collect();
    InlineKeyboardMarkup::new(rows)
}

