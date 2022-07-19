use teloxide::prelude::*;
use crate::ui::{self, MyDialogue, HandlerResult};
use crate::{Chat, Db};

pub async fn list_my_assignments(
    bot: AutoSend<Bot>,
    db: Db,
    pcid: ChatId,
    chat: &Chat,
    uid: UserId,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> list_my_assignments");
    let orders = db.clone().active_assignments_to(pcid, uid).await?;
    if orders.is_empty() {
        bot.send_message(dialogue.chat_id(), "No assigned orders")
            .await?;
    } else {
        bot.send_message(dialogue.chat_id(), "Orders assigned to you:").await?;
        let uid = match chat.is_private() {
            true => Some(uid),
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

