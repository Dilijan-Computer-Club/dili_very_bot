use crate::Db;
use crate::ui::{self, HandlerResult};
use teloxide::{
    prelude::*,
    utils::command::BotCommands,
};

#[derive(BotCommands, Clone)]
#[command(rename = "snake_case",
          description = "These commands are supported:")]
pub enum Command {
    #[command(description = "Start here")]
    Start,
    #[command(description = "Show main menu")]
    Menu,
    #[command(description = "Debugging, this should be deleted")]
    Debug,
    #[command(description = "Show how to use me")]
    Help,
    #[command(description = "Get the bot to know you")]
    Hello,
    #[command(description = "What the bot knows about you (mostly debugging)")]
    Me,
}

/// Show some debugging info
///
/// TODO permissions
async fn debug_msg(
    bot: AutoSend<Bot>,
    msg: Message,
    mut db: Db,
) -> HandlerResult {
    let s = db.debug_stats().await?;
    bot.send_message(msg.chat.id, s).await?;
    Ok(())
}

pub async fn handle_command(
    bot: AutoSend<Bot>,
    msg: Message,
    command: Command,
    db: Db,
) -> HandlerResult {
    let msg_id = msg.id;
    let user = msg.from();
    let cid = msg.chat.id;
    match command {
        Command::Start => { ui::main_menu::main_menu(bot.clone(), cid).await? },
        Command::Menu  => { ui::main_menu::main_menu(bot.clone(), cid).await? },
        Command::Debug => { debug_msg(bot.clone(), msg, db).await? },
        Command::Hello => { ui::say_hello::say_hello(bot.clone(), cid, msg.from()).await? },
        Command::Help =>  { bot.clone().send_message(cid, ui::help::help()).await?; },
        Command::Me =>  { ui::me::send_me(bot.clone(), db, cid, user).await?; },
    }
    // Delete the command message after the command is handled
    bot.delete_message(cid, msg_id).await?;
    Ok(())
}

