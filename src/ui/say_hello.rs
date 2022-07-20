use teloxide::{
    prelude::*,
    types::User,
};
use crate::HandlerResult;
use crate::ui;
use crate::utils;
use crate::markup;
use std::time::Duration;

/// Delete our "hello" sent in a public that after this amount
const OUR_HELLO_DEL_TIMEOUT: Duration = Duration::from_millis(10_000);

pub async fn say_hello(
    bot: AutoSend<Bot>,
    cid: ChatId,
    user: Option<&User>,
) -> HandlerResult {
    if cid.is_user() {
        bot.send_message(cid, "Send this message in a public chat, \
so I know you're there.").await?;
        return Ok(());
    }

    let sent: Message =
        if let Some(user) = user {
            // Confirm that we've received it
            let sent = {
                let bot = bot.clone()
                    .parse_mode(teloxide::types::ParseMode::Html);
                let mention = markup::user_link(user);
                let msg = format!("{mention} See you in a private chat!");
                let sent: Message = bot.send_message(cid, msg).await?;
                sent
            };
            // Now say hello in a private chat
            let help = ui::help::help();
            let msg =
                format!("Hi there! Here is how you can talk to me:\n{help}");
            bot.send_message(utils::uid_to_cid(user.id), msg).await?;
            sent
        } else {
            bot.send_message(cid, "Hi there!").await?
        };

    let msg_id = sent.id;
    tokio::spawn(async move {
        tokio::time::sleep(OUR_HELLO_DEL_TIMEOUT).await;
        // I don't care if it fails, it's no biggie
        let _ = bot.delete_message(cid, msg_id).await;
    });

    Ok(())
}
