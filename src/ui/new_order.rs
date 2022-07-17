use teloxide::{
    prelude::*,
    dispatching::UpdateHandler,
};

use crate::db::Db;
use crate::order::Order;

use crate::error::Error;

type HandlerResult = Result<(), Error>;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start, // receive name
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
    db.debug_stats().await?;
    let pcid = db.pub_chat_id_from_msg(msg.clone()).await;
    let pcid = match pcid {
        Err(e) => {
            log::warn!(" -> recv_desc: Could not get pcid from msg {:?}", &msg);
            bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
            return Err(format!("{e}").into());
        },
        Ok(pcid) => pcid,
    };

    if let Some(text) = msg.text() {
        log::info!("Description: {text}");
        let mut order = Order::from_tg_msg(&msg)?;
        let order_id = db.add_order(pcid, &order).await?;
        order.id = Some(order_id);

        let uid = match msg.chat.is_private() {
            true  => Some(order.from.id),
            false => None,
        };

        order.send_message_for(
            &mut bot,
            uid,
            msg.chat.id).await?;
        dialogue.update(crate::State::Start).await?;
    } else {
        bot.send_message(dialogue.chat_id(), "No description").await?;
    }

    Ok(())
}
