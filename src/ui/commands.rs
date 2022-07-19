use crate::{Db};
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
    #[command(description = "Debugging")]
    Debug,
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
    match command {
        Command::Start => { ui::main_menu::main_menu(bot, msg.chat.id).await? },
        Command::Menu  => { ui::main_menu::main_menu(bot, msg.chat.id).await? },
        Command::Debug => { debug_msg(bot, msg, db).await? },
    }
    Ok(())
}

