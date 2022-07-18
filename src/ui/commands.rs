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
    #[command(description = "Show help")]
    Help,
    #[command(description = "Debugging")]
    Debug,
    #[command(description = "List active orders")]
    ListActiveOrders,
    #[command(description = "Show my orders")]
    ListMyOrders,
    #[command(description = "Make New Order")]
    MakeNewOrder,
}

/// Show some debugging info
///
/// TODO limit the displayed information to what's allowed
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
        Command::Start => { ui::main_menu::main_menu(bot, msg, db).await? },
        Command::Menu  => { ui::main_menu::main_menu(bot, msg, db).await? },
        Command::Help => {},
        Command::Debug => { debug_msg(bot, msg, db).await? },
        Command::ListActiveOrders => {},
        Command::ListMyOrders => {},
        Command::MakeNewOrder => {},
    }
    Ok(())
}

