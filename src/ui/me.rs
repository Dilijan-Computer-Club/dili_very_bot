use crate::Db;
use teloxide::{
    prelude::*,
    types::{User},
};
use crate::HandlerResult;
use std::fmt::Write;

/// Shows basic information about the user
pub async fn send_me(
    bot: AutoSend<Bot>,
    mut db: Db,
    cid: ChatId,
    user: Option<&User>
) -> HandlerResult {
    if user.is_none() {
        bot.send_message(cid, "I don't know who sent this message. Thanks, Telegram!")
            .await?;
        return Ok(())
    }
    let user = user.unwrap();

    let pub_chats: Vec<(ChatId, String)> =
        db.user_public_chats(user.id).await?;

    let mut ret = "I know that you're in the following chats:\n\n".to_string();
    if pub_chats.is_empty() {
        ret.push_str("Actually I don't see you in any chat. \
Try saying /hello to a public chat I'm in");
    }

    for (_cid, name) in pub_chats.into_iter() {
        writeln!(&mut ret, " - {name}")?;
    }
    bot.send_message(cid, ret).await?;

    Ok(())
}
