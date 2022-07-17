use tokio::task::spawn_blocking;
use teloxide::prelude::*;
use teloxide::types::{User, Chat, MessageKind, MessageCommon, ChatKind,
                      MessageNewChatMembers, MessageLeftChatMember};
use std::sync::{Arc, RwLock};
use std::fmt;
use std::collections::BTreeMap;
use crate::error::Error;
use crate::order::{self, Order, OrderId, Action, ActionKind, Status};
use crate::order::ActionError;

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
        // write!(f, "({}, {})", self.x, self.y)
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

/// Wrapper for InnerDb that is Send, Sync, and async
#[derive(Clone)]
pub struct Db {
    db: Arc<RwLock<InnerDb>>,
}

impl Db {
    pub fn new() -> Self {
        Db { db: Arc::new(RwLock::new(InnerDb::default())) }
    }

    pub async fn add_order(
        &mut self,
        pcid: ChatId,
        order: &Order
    ) -> Result<OrderId, Error> {
        let db = self.db.clone();
        let order = order.clone();
        spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.add_order(pcid, order)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn debug_stats(&self) -> Result<String, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            Ok(db.debug_stats())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn get_priv_chat_id(&self, uid: UserId) -> Result<Option<ChatId>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            Ok(db.get_priv_chat_id(uid))
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn get_user(&self, uid: UserId) -> Result<Option<User>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            Ok(db.get_user(uid).cloned())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }
    pub async fn pub_chat_id_from_msg(
        &self,
        msg: Message,
    ) -> Result<ChatId, PubChatFromMsgError> {
        let db = self.db.clone();
        let ret = spawn_blocking(move || {
            let db = db.read();
            if let Err(e) = db {
                log::warn!("RLock: {e:?}");
                return Err(PubChatFromMsgError::Other);
            }
            let db = db.unwrap();
            db.pub_chat_id_from_msg(msg)
        }).await;
        match ret {
            Ok(ret) => ret,
            Err(e) => {
                log::warn!("pub_chat_id_from_msg error: {e:?}");
                Err(PubChatFromMsgError::Other)
            }
        }
    }

    pub async fn orders_by_status(
        &self,
        pcid: ChatId,
        status: Status,
    ) -> Result<Vec<Order>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            db.orders_by_status(pcid, status)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn active_assignments_to(
        &self,
        pcid: ChatId,
        uid: UserId
    ) -> Result<Vec<Order>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            db.active_assignments_to(pcid, uid)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn orders_submitted_by_user(
        &self,
        pcid: ChatId,
        uid: UserId
    ) -> Result<Vec<Order>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            db.orders_submitted_by_user(pcid, uid)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    /// Performs the action and returns previous state and the Order
    /// If the order is deleted then the returned order is None
    pub async fn perform_action(
        &mut self,
        uid: UserId,
        pcid: ChatId,
        action: Action,
    ) -> Result<(order::Status, Option<Order>), ActionError> {
        let db = self.db.clone();
        let res = spawn_blocking(move || {
            let db = db.write();
            if let Err(e) = db {
                log::warn!("WLock: {e:?}");
                return Err(ActionError::Other);
            }
            let mut db = db.unwrap();
            db.perform_action(uid, pcid, &action)
        }).await;

        match res {
            Ok(res) => res,
            Err(e) => {
                log::warn!("Error while performing action: {e:?}");
                Err(ActionError::Other)
            }
        }
    }

    pub async fn collect_data_from_msg(
        &mut self,
        msg: Message,
    ) -> Result<(), Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.collect_data_from_msg(msg)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn update_user(
        &mut self,
        user: User,
    ) -> Result<(), Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.update_user(user);
            Ok(())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn update_chat(
        &mut self,
        chat: Chat,
        user: Option<User>,
    ) -> Result<(), Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.update_chat(chat, user.as_ref());
            Ok(())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }
}

#[derive(Clone, Debug)]
struct PublicChat {
    chat: Chat,
    members: Vec<UserId>,
    orders: Vec<Order>,
}

impl PublicChat {
    fn new(chat: Chat) -> PublicChat {
        PublicChat {
            chat,
            members: Vec::new(),
            orders: Vec::new(),
        }
    }

    fn add_user(&mut self, uid: UserId) {
        log::debug!("-> add_user uid {uid} to chat {}",
                    self.chat.title().unwrap_or("<noname>"));
        if ! self.members.iter_mut().any(|id| *id == uid) {
            log::debug!("adding uid {uid} to chat {}",
                        self.chat.title().unwrap_or("<noname>"));
            self.members.push(uid);
        }
    }
}

#[derive(Debug)]
struct InnerDb {
    max_id: OrderId,
    /// list of known public chats and their members
    public_chats: Vec<PublicChat>,
    /// list of our private chats with users
    private_chats: Vec<(ChatId, UserId)>,
    users: BTreeMap<UserId, User>,
}

impl Default for InnerDb {
    fn default() -> Self {
        InnerDb {
            public_chats: Vec::new(),
            private_chats: Vec::new(),
            users: BTreeMap::new(),
            max_id: OrderId(0),
        }
    }
}

impl InnerDb {
    fn next_id(&mut self) -> OrderId {
        self.max_id.0 += 1;
        self.max_id
    }

    fn pub_chat(&self, cid: ChatId) -> Option<&PublicChat> {
        self.public_chats.iter()
            .find(|pc| pc.chat.id == cid)
    }

    fn pub_chat_mut(&mut self, cid: ChatId) -> Option<&mut PublicChat> {
        self.public_chats.iter_mut()
            .find(|pc| pc.chat.id == cid)
    }

    pub fn add_order(
        &mut self,
        pub_chat_id: ChatId,
        mut order: Order
    ) -> Result<OrderId, Error> {
        let new_id = self.next_id();
        let pub_chat = self.pub_chat_mut(pub_chat_id);
        if pub_chat.is_none() {
            return Err(format!(
                    "could not find public chat {pub_chat_id}").into())
        }
        let pub_chat = pub_chat.unwrap();
        order.id = Some(new_id);
        let s =
            format!("Added order {order:?} new id = {}", new_id.0);
        pub_chat.orders.push(order);
        log::info!("{}", s);
        Ok(new_id)
    }

    pub fn orders_by_status(
        &self,
        pub_chat_id: ChatId,
        status: Status,
    ) -> Result<Vec<Order>, Error> {
        log::info!("Listing orders of status {status}");

        let pub_chat = self.pub_chat(pub_chat_id);
        if pub_chat.is_none() {
            return Err(format!(
                    "could not find public chat {pub_chat_id}").into())
        }
        let pub_chat = pub_chat.unwrap();

        Ok(pub_chat.orders.iter()
            .filter(|o| o.status() == status)
            .cloned()
            .collect())
    }

    pub fn orders_submitted_by_user(
        &self,
        pub_chat_id: ChatId,
        uid: UserId
    ) -> Result<Vec<Order>, Error> {
        log::info!("Listing orders submitted by user {:?}", uid);
        let pub_chat = self.pub_chat(pub_chat_id);
        if pub_chat.is_none() {
            return Err(format!(
                    "could not find public chat {pub_chat_id}").into())
        }
        let pub_chat = pub_chat.unwrap();
        Ok(pub_chat.orders.iter()
            .filter(|o| o.from.id == uid)
            .cloned()
            .collect())
    }

    pub fn active_assignments_to(
        &self,
        pub_chat_id: ChatId,
        uid: UserId,
    ) -> Result<Vec<Order>, Error> {
        log::info!("Listing orders assigned to {:?}", uid);

        let pub_chat = self.pub_chat(pub_chat_id);
        if pub_chat.is_none() {
            return Err(format!(
                    "could not find public chat {pub_chat_id}").into())
        }
        let pub_chat = pub_chat.unwrap();

        Ok(pub_chat.orders.iter()
           .filter(|o| o.is_active_assignment())
           .filter(|o| {
               if let Some((_when, assignee_id, _u)) = &o.assigned {
                    *assignee_id == uid
               } else {
                   false
               }
           })
           .cloned()
           .collect())
    }

    fn find_order(
        &mut self,
        pub_chat_id: ChatId,
        order_id: OrderId
    ) -> Option<&Order> {
        let pub_chat = self.pub_chat_mut(pub_chat_id);
        if pub_chat.is_none() {
            log::warn!("Could not find public chat id {pub_chat_id}");
            return None
        }
        let pub_chat = pub_chat.unwrap();
        pub_chat.orders.iter().find(|o| o.id == Some(order_id) )
    }

    fn find_order_mut(
        &mut self,
        pub_chat_id: ChatId,
        order_id: OrderId
    ) -> Option<&mut Order> {
        let pub_chat = self.pub_chat_mut(pub_chat_id);
        if pub_chat.is_none() {
            log::warn!("Could not find public chat id {pub_chat_id}");
            return None
        }
        let pub_chat = pub_chat.unwrap();
        pub_chat.orders.iter_mut().find(|o| o.id == Some(order_id) )
    }

    /// Deletes the order and returns its previous status
    fn delete_order(
        &mut self,
        pub_chat_id: ChatId,
        order_id: OrderId
    ) -> Result<order::Status, ActionError> {
        let pub_chat = self.pub_chat_mut(pub_chat_id);
        if pub_chat.is_none() {
            return Err(ActionError::PubChatNotFound)
        }
        let pub_chat = pub_chat.unwrap();

        for (ii, order) in pub_chat.orders.iter().enumerate() {
            if order.id == Some(order_id) {
                let status = order.status();
                pub_chat.orders.remove(ii);
                return Ok(status);
            }
        }
        Err(ActionError::OrderNotFound(order_id))
    }

    /// performs the action, returns modified order if successful
    pub fn perform_action(
        &mut self,
        uid: UserId,
        pub_chat_id: ChatId,
        action: &Action,
    ) -> Result<(order::Status, Option<Order>), ActionError> {

        if action.kind == ActionKind::Delete {
            let order = self.find_order(pub_chat_id, action.order_id)
                .ok_or(ActionError::OrderNotFound(action.order_id))?;
            if ! order.is_action_permitted(uid, action) {
                return Err(ActionError::NotPermitted)
            }

            let status = self.delete_order(pub_chat_id, action.order_id)?;
            Ok((status, None))
        } else {
            let order = self.find_order_mut(pub_chat_id, action.order_id)
                .ok_or(ActionError::OrderNotFound(action.order_id))?;
            let prev_status = order.perform_action(uid, action)?;
            Ok((prev_status, Some(order.clone())))
        }
    }

    pub fn get_priv_chat_id(&self, uid: UserId) -> Option<ChatId> {
        self.private_chats.iter()
            .find(|(_, cuid)| *cuid == uid)
            .map(|(cid, _)| cid)
            .cloned()
    }

    pub fn get_user(&self, uid: UserId) -> Option<&User> {
        self.users.get(&uid)
    }

    /// Try to figure out which public chat the message belongs to
    pub fn pub_chat_id_from_msg(
        &self,
        msg: Message
    ) -> Result<ChatId, PubChatFromMsgError> {
        // if msg.is public then that's it
        // otherwise if user is present find them in chats, if there is one
        //   then that's it
        // return appropriate error otherwise
        if let ChatKind::Public(_) = msg.chat.kind {
            return Ok(msg.chat.id)
        }

        let user = match msg.from() {
            Some(user) => user,
            None => {
                log::warn!("No user in message {msg:?}");
                return Err(PubChatFromMsgError::Other)
            },
        };

        let mut pub_chats = self.public_chats.iter()
            .filter(|pc| pc.members.iter().any(|u| *u == user.id));
        let pc = pub_chats.next();
        if pc.is_none() { return Err(PubChatFromMsgError::NotInPubChats) }
        if pub_chats.next().is_some() {
            return Err(PubChatFromMsgError::MultipleChats)
        }
        Ok(pc.unwrap().chat.id)
    }

    pub fn collect_data_from_msg(
        &mut self,
        msg: Message
    ) -> Result<(), Error> {
        log::debug!("-> collect_data_from_msg, {msg:?}");

        if let Some(user) = msg.from()  {
            self.update_user(user.clone());
        }
        let cid = msg.chat.id;
        self.update_chat(msg.clone().chat, msg.from());
        match msg.kind {
            MessageKind::NewChatMembers(payload) => {
                self.handle_new_chat_members(cid, &payload);
            },
            MessageKind::LeftChatMember(payload) => {
                self.handle_left_chat_members(cid, &payload);
            },
            MessageKind::Common(payload) => {
                self.gather_data_from_common_msg(&payload);
            },
            _ => {},
        }
        Ok(())
    }

    fn gather_data_from_common_msg(&mut self, m: &MessageCommon) {
        if let Some(chat) = &m.sender_chat {
            self.update_chat(chat.clone(), m.from.as_ref());
        }
    }

    fn handle_new_chat_members(
        &mut self,
        pcid: ChatId,
        m: &MessageNewChatMembers
    ) {
        for u in m.new_chat_members.iter() {
            self.update_user(u.clone());
        }

        let pc = self.pub_chat_mut(pcid);
        if pc.is_none() { return }
        let pc = pc.unwrap();

        for u in m.new_chat_members.iter() {
            if ! pc.members.iter().any(|m| *m == u.id) {
                pc.members.push(u.id);
            }
        }
    }

    fn handle_left_chat_members(
        &mut self,
        cid: ChatId,
        m: &MessageLeftChatMember
    ) {
        let user = &m.left_chat_member;
        let uid = user.id;
        self.update_user(user.clone());
        let pc = self.public_chats.iter_mut().find(|c| c.chat.id == cid);
        if pc.is_none() {
            log::warn!("could not find chat {cid} for left user {}", uid);
            return;
        }
        if pc.is_none() {
            return
        }
        let pc = pc.unwrap();

        let ii = pc.members.iter().enumerate()
            .find(|(_, u)| **u == user.id)
            .map(|(ii, _)| ii);
        if let Some(ii) = ii {
            pc.members.remove(ii);
        }

    }

    pub fn update_user(&mut self, user: User) {
        if let Some(u) = self.users.get_mut(&user.id) {
            *u = user.clone();
        } else {
            self.users.insert(user.id, user);
        }
    }

    fn update_chat(&mut self, chat: Chat, user: Option<&User>) {
        match chat.kind {
            ChatKind::Public(_) => {
                let c =
                    self.public_chats.iter_mut()
                        .find(|c| c.chat.id == chat.id);
                if let Some(c) = c {
                    // Update chat
                    c.chat = chat;
                    if let Some(user) = user {
                        c.add_user(user.id);
                    }
                } else {
                    // chat doesn't exist yet, so save it
                    let mut chat = PublicChat::new(chat);
                    if let Some(user) = user {
                        chat.members.push(user.id);
                    }
                    self.public_chats.push(chat);
                }

            },
            ChatKind::Private(_) => {
                if ! self.private_chats.iter().any(|(cid, _uid)| *cid == chat.id) {
                    // doesn't exist, so create it
                    if let Some(user) = user {
                        self.private_chats.push((chat.id, user.id));
                    }
                }
            },
        }
    }

    pub fn debug_stats(&self) -> String {
        let max_id = self.max_id;
        let num_private_chats = self.private_chats.len();
        let num_public_chats = self.public_chats.len();
        let num_users = self.users.len();

        let mut users = String::new();
        for u in self.users.values() {
            let name = format!(
                "@{} {}, ",
                u.username.clone().unwrap_or_else(|| "<noname>".to_string()),
                u.first_name);
            users.push_str(name.as_ref());
        }

        let mut pub_chats = String::new();
        let mut num_orders = 0;
        for pc in self.public_chats.iter() {
            let name = pc.chat.title().unwrap_or("<noname>");
            let num_users = pc.members.len().to_string();
            let mut users = String::new();
            num_orders += pc.orders.len();
            for uid in pc.members.iter() {
                let u = self.users.get(uid).unwrap();
                let s = format!("{}, ", u.username.clone().unwrap_or_else(|| "<noname>".to_string()));
                users.push_str(&s);
            }
            let num_orders = pc.orders.len();
            let s = format!("{name} {num_users} users, {num_orders} orders, users: {users} ");
            pub_chats.push_str(&s);
        }
        let s = format!("\
Data:
max_id =       {max_id}
Private chats: {num_private_chats}
Public chats:  {num_public_chats}
Users:         {num_users}
Orders:        {num_orders}

Users:
{users}

Public chats:
{pub_chats}
");
        log::info!("{s}");
        s
    }
}
