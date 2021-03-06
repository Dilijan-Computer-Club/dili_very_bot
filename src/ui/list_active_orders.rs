use teloxide::prelude::*;
use crate::{Db, Chat};
use crate::ui::{self, HandlerResult};
use crate::order;

pub async fn list_active_orders(
    bot: AutoSend<Bot>,
    db: Db,
    pcid: ChatId,
    chat: &Chat,
    uid: UserId,
) -> HandlerResult {
    log::info!("-> list_active_orders");
    let cid = chat.id;
    let orders = db.clone()
        .orders_by_status(pcid, order::Status::Published).await?;
    if orders.is_empty() {
        ui::text_msg(Some(ui::TEMP_MSG_TIMEOUT), bot, cid, "No active orders").await?;
    } else {
        ui::text_msg(Some(ui::TEMP_MSG_TIMEOUT), bot.clone(), cid,
                          "All active orders:").await?;
        let uid = match chat.is_private() {
            true =>  Some(uid),
            false => None,
        };
        let msg: Option<&str> = None;
        for order in orders.iter() {
            ui::order::send_message(
                db.clone(), order, bot.clone(), uid, chat.id, msg).await?;
        }
    }
    Ok(())
}

