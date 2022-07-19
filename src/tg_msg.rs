use teloxide::prelude::*;
use serde::{Serialize, Deserialize};

/// TODO Probably don't need it
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TgMsg {
    pub chat_id: ChatId,
    pub message_id: i32,
    pub text: String,
}

impl AsRef<str> for TgMsg {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

