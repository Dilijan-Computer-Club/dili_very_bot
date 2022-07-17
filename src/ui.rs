use teloxide::{
    prelude::*,
    dispatching::dialogue::InMemStorage,
};

mod collect_data;
pub use collect_data::collect_data;

pub mod main_menu;

pub mod new_order;

mod show_my_orders;
pub use show_my_orders::show_my_orders;

use crate::error::Error;
pub type HandlerResult = Result<(), Error>;
pub type MyDialogue = Dialogue<State, InMemStorage<State>>;
pub type MyStorage = InMemStorage<State>;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    NewOrder(new_order::State)
}


pub async fn pcid_or_err(bot: &AutoSend<Bot>, db: &crate::Db,
    msg: &Message, dialogue: &MyDialogue
) -> Result<ChatId, Error> {
    let pcid = db.pub_chat_id_from_msg(msg.clone()).await;
    match pcid {
        Ok(pcid) => Ok(pcid),
        Err(e) => {
            log::warn!("-> handle_callback_query pcid: {e:?}");
            bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
            Err(format!("{e:?}").into())
        }
    }
}