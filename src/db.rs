pub mod mem;
pub mod redis_db;

pub type Db = redis_db::Db;
// pub type Db = mem::Db;

#[derive(Clone, Copy, Debug)]
pub enum PubChatFromMsgError {
    /// We don't see the user in any public chats
    NotInPubChats,

    /// User is in multiple chats, so we need to ask which one they want
    MultipleChats,

    /// Other technical error, like we couldn't access the db or something
    Other,
}

