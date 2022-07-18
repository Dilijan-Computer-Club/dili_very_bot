use crate::Db;
use crate::ui::{self, HandlerResult, MyDialogue, State};
use teloxide::{
    prelude::*,
    types::Chat,
};

pub async fn show_my_orders(
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
        let msg: Option<&str> = None;
        for order in orders.iter() {
            ui::order::send_message(order, &mut bot, uid, chat.id, msg).await?;
        }
    }
    dialogue.update(State::Start).await?;
    Ok(())
}

