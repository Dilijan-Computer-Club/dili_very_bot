use std::borrow::Cow;
use crate::ui::commands;
use teloxide::utils::command::BotCommands;

pub fn help() -> Cow<'static, str> {
    let cmds = commands::Command::descriptions();
    format!("
You can post and manage orders from a private chat with me and get notifications in a public chat.
Here is my commands:
{cmds}
").into()
}
