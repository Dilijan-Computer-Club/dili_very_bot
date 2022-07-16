use teloxide::{
    prelude::*,
    payloads::SendMessageSetters,
    types::{
        Chat, ChatKind,
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
    let message_handler = Update::filter_message()
        .branch(dptree::case![State::Start]
                .endpoint(receive_description));
    message_handler
}

pub async fn send_initial_message(
    bot: AutoSend<Bot>,
    chat_id: ChatId)
-> HandlerResult {
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
    db.print_stats();
    let pcid = db.pub_chat_id_from_msg(msg.clone()).await?;
    if pcid.is_none() {
        log::warn!("Could not get pcid from msg {:?}", &msg);
        return Ok(())
    }
    let pcid = pcid.unwrap();

    if let Some(text) = msg.text() {
        log::info!("Description: {text}");
        let mut order = Order::from_tg_msg(&msg)?;
        let order_id = db.add_order(pcid, &order).await?;
        order.id = Some(order_id);

        order.send_message_for(
            &mut bot,
            order.from.id,
            msg.chat.id,
            matches!(msg.chat.kind, ChatKind::Public(_))
        ).await?;
        dialogue.update(crate::State::Start).await?;
    } else {
        bot.send_message(dialogue.chat_id(), "No description").await?;
    }

    Ok(())
}
