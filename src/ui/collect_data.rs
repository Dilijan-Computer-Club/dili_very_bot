use crate::db::Db;
use crate::ui::HandlerResult;
use teloxide::{
    prelude::*,
    types::UpdateKind,
};
async fn collect_data_from_msg(
    mut db: Db,
    msg: Message,
) -> HandlerResult {
    db.collect_data_from_msg(msg).await?;
    Ok(())
}

pub async fn collect_data(
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

