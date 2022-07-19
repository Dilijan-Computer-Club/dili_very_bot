pub mod mem;
pub mod redis_db;

use std::fmt;

#[cfg(feature = "redis_db")]
pub type Db = redis_db::Db;
#[cfg(feature = "mem_db")]
pub type Db = mem::Db;

#[derive(Clone, Copy, Debug)]
pub enum PubChatFromMsgError {
    /// We don't see the user in any public chats
    NotInPubChats,

    /// User is in multiple chats, so we need to ask which one they want
    MultipleChats,

    /// Other technical error, like we couldn't access the db or something
    Other,
}

impl fmt::Display for PubChatFromMsgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PubChatFromMsgError::NotInPubChats => {
                write!(f, "You are not in any public chat with this bot.
Try writting '/hello' into the public chat you're in to make sure \
the bot knows you're there")
            },
            PubChatFromMsgError::MultipleChats => {
                // TODO support for multiple chats per user
                write!(f, "You are in multiple chats. This is not supported yet. Sorry!")
            },
            PubChatFromMsgError::Other => {
                write!(f, "Some error occured")
            }
        }
    }
}

