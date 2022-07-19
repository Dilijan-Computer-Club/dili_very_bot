
async fn say_hello(
    bot: AutoSend<Bot>,
    cid: ChatId,
    user: Option<&User>,
) -> HandlerResult {
    if cid.is_user() {
        bot.send_message(cid, "Send this message in a public chat, \
so I know you're there.").await?;
        return Ok(());
    }
    if let Some(user) = user {
        // Confirm we've received it
        let id = bot.send_message(cid, "See you in a private chat!").await?;
    }

    
    Ok(())
}
