use teloxide::{
    prelude::*,
    payloads::SendMessageSetters,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup,
        InlineQueryResultArticle, InputMessageContent,
    },
    dispatching::{
        dialogue::{self, InMemStorage, Storage},
        UpdateHandler
    },
    utils::command::BotCommands,
};

use crate::urgency::Urgency;
use crate::db::Db;
use crate::Order;

use crate::error::Error;

type HandlerResult = Result<(), Error>;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start, // receive name
    DescReceived { name: String }
}

pub fn schema() -> UpdateHandler<Error> {
    Update::filter_message()
        .branch(dptree::case![State::Start]
                .endpoint(receive_description))
}

pub async fn send_initial_message(bot: AutoSend<Bot>, chat_id: ChatId) -> HandlerResult {
    bot.send_message(chat_id, "What do you want?").await?;
    Ok(())
}

async fn receive_description(
    mut bot: AutoSend<Bot>,
    msg: Message,
    mut db: Db,
    dialogue: crate::MyDialogue,
) -> HandlerResult {
    log::info!("-> receive_description");
    if let Some(text) = msg.text() {
        // bot.send_message(dialogue.chat_id(),
        //     format!("Your message: {text}")).await?;
        log::info!("Description: {text}");
        let order = Order::from_tg_msg(&msg)?;
        db.add_order(&order)?;
        order.send_message_for(&mut bot, order.from.id, msg.chat.id).await?;
        dialogue.update(crate::State::Start).await?;
        // dialogue.exit().await?;
    } else {
        bot.send_message(dialogue.chat_id(), "No description").await?;
    }

    Ok(())
}
