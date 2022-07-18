use teloxide::types::{UserId, Chat};
use crate::order::Order;

#[derive(Clone, Debug)]
pub struct PublicChat {
    pub chat: Chat,
    pub members: Vec<UserId>,
    pub orders: Vec<Order>,
}

impl PublicChat {
    pub fn new(chat: Chat) -> PublicChat {
        PublicChat {
            chat,
            members: Vec::new(),
            orders: Vec::new(),
        }
    }

    pub fn add_user(&mut self, uid: UserId) {
        log::debug!("-> add_user uid {uid} to chat {}",
                    self.chat.title().unwrap_or("<noname>"));
        if ! self.members.iter_mut().any(|id| *id == uid) {
            log::debug!("adding uid {uid} to chat {}",
                        self.chat.title().unwrap_or("<noname>"));
            self.members.push(uid);
        }
    }

    pub fn has_user(&self, uid: UserId) -> bool {
        self.members.iter().any(|u| *u == uid)
    }
}

