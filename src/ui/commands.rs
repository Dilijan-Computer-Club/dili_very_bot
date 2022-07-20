use crate::Db;
use crate::ui::{self, HandlerResult};
use crate::MyDialogue;
use teloxide::{
    prelude::*,
    utils::command::BotCommands,
};
use std::fmt;

#[derive(BotCommands, Clone)]
#[command(rename = "snake_case",
          description = "These commands are supported:")]
pub enum Command {
    #[command(description = "Start here")]
    Start,
    #[command(description = "Show main menu")]
    Menu,
    #[command(description = "Show how to use me")]
    Help,
    #[command(description = "Create a new order")]
    NewOrder,
    #[command(description = "Get the bot to know you")]
    Hello,
    #[command(description = "What the bot knows about you (mostly debugging)")]
    Me,
    #[command(description = "Debugging, this should be deleted")]
    Debug,
}

impl Command {
    /// Function for printing a command
    pub const fn cmd(&self) -> &'static str {
        match self {
            Command::Start    => "/start",
            Command::Menu     => "/menu",
            Command::Help     => "/help",
            Command::NewOrder => "/new_order",
            Command::Hello    => "/hello",
            Command::Me       => "/me",
            Command::Debug    => "/debug",
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.cmd())
    }
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
    dialogue: MyDialogue,
    msg: Message,
    command: Command,
    db: Db,
) -> HandlerResult {
    let msg_id = msg.id;
    let user = msg.from();
    let cid = msg.chat.id;
    match command {
        Command::Start    => { ui::main_menu::main_menu(bot.clone(), cid).await? },
        Command::Menu     => { ui::main_menu::main_menu(bot.clone(), cid).await? },
        Command::Debug    => { debug_msg(bot.clone(), msg, db).await? },
        Command::Hello    => { ui::say_hello::say_hello(bot.clone(), cid, msg.from()).await? },
        Command::Help     => { bot.clone().send_message(cid, ui::help::help()).await?; },
        Command::Me       => { ui::me::send_me(bot.clone(), db, cid, user).await?; },
        Command::NewOrder => {
            if let Some(user) = user {
                ui::new_order::start(
                    bot.clone(), db, dialogue, cid, user.id).await?
            } else {
                log::warn!("/new_order: could get user from msg {msg:?}");
                bot.send_message(cid, "We don't know who you are. Thanks Telegram!
Anyway, please try again and it should work.").await?;
            }
        }
    }

    // Delete the command message after the command is handled
    bot.delete_message(cid, msg_id).await?;
    Ok(())
}

