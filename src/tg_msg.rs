use teloxide::prelude::*;
use serde::{Serialize, Deserialize};

use crate::error::Error;

/// TODO Probably don't need it
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TgMsg {
    pub chat_id: ChatId,
    pub message_id: i32,
    pub text: String,
}

impl TgMsg {
    pub fn from_tg_msg(msg: &Message) -> Result<TgMsg, Error> {
        let text: String =
            match msg.text() {
                Some(text) => text.to_string(),
                None => return Err("Message has no text".into()),
            };

        Ok(TgMsg {
            chat_id: msg.chat.id,
            message_id: msg.id,
            text,
        })
    }
}

impl AsRef<str> for TgMsg {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

