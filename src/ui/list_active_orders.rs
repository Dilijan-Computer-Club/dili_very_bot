use teloxide::prelude::*;
use crate::{Db, Chat};
use crate::ui::{self, MyDialogue, HandlerResult};
use crate::order;

pub async fn list_active_orders(
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
            ui::order::send_message(&order, &mut bot, uid, chat.id).await?;
        }
    }
    Ok(())
}

