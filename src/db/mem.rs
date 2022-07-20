#![cfg(feature = "mem_db")]

use tokio::task::spawn_blocking;
use teloxide::prelude::*;
use teloxide::types::{User, UserId, Chat, ChatKind, MessageId};
use std::sync::{Arc, RwLock};
use std::collections::{BTreeSet, BTreeMap};
use crate::error::Error;
use crate::order::{self, Order, OrderId, Action, ActionKind, Status};
use crate::order::ActionError;


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

    pub fn remove_user(&mut self, uid: UserId) {
        for (ii, id) in self.members.iter().enumerate() {
            if *id == uid {
                self.members.remove(ii);
                break;
            }
        }
    }

    pub fn has_user(&self, uid: UserId) -> bool {
        self.members.iter().any(|u| *u == uid)
    }
}

/// Wrapper for InnerDb that is Send, Sync, and async
#[derive(Clone)]
pub struct Db {
    db: Arc<RwLock<InnerDb>>,
}

impl Db {
    pub async fn new() -> Result<Self, Error> {
        Ok(Db { db: Arc::new(RwLock::new(InnerDb::default())) })
    }

    pub async fn user_public_chats(
        &mut self,
        uid: UserId,
    ) -> Result<Vec<(ChatId, String)>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            Ok(db.public_chats.iter()
                .filter(|pc| pc.has_user(uid))
                .cloned()
                .map(|pc| (pc.chat.id, pc.chat.title().unwrap().to_string()))
                .collect())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    /// Returns new order's `OrderId`
    pub async fn add_order(
        &mut self,
        pcid: ChatId,
        order: &mut Order
    ) -> Result<OrderId, Error> {
        let db = self.db.clone();
        let mut o = order.clone();
        let oid = spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.add_order(pcid, &mut o)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()?;
        order.id = Some(oid);
        Ok(oid)
    }

    pub async fn debug_stats(&mut self) -> Result<String, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            Ok(db.debug_stats())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn get_user(&mut self, uid: UserId) -> Result<Option<User>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            Ok(db.get_user(uid).cloned())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn orders_by_status(
        &mut self,
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
        &mut self,
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
        &mut self,
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
        user: User,
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
            db.perform_action(user, pcid, &action)
        }).await;

        match res {
            Ok(res) => res,
            Err(e) => {
                log::warn!("Error while performing action: {e:?}");
                Err(ActionError::Other)
            }
        }
    }

    pub async fn add_members(
        &mut self,
        cid: ChatId,
        uids: Vec<UserId>,
    ) -> Result<(), Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.add_members(cid, uids.iter().cloned());
            Ok(())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn remove_chat_membership(
        &mut self,
        cid: ChatId,
        uid: UserId,
    ) -> Result<(), Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.remove_member(cid, uid);
            Ok(())
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
    ) -> Result<(), Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.update_chat(chat)?;
            Ok(())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    /// Get which messages we've sent that contain this order
    ///
    /// Returns pairs of (chat_id, order_id) because we need `chat_id` to
    /// change or delete these messages
    pub async fn order_msg_ids(
        &mut self,
        oid: OrderId,
    ) -> Result<Vec<(ChatId, MessageId)>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || -> Result<Vec<(ChatId, MessageId)>, Error> {
            let db = db.read().map_err(|e| format!("lock: {e:?}"))?;
            let ret = db.order_msgs.get(&oid);
            if ret.is_none() {
                return Ok(Vec::new())
            }
            let ret = ret.unwrap();
            let ret: Vec<(ChatId, MessageId)> =
                ret.iter()
                .map(|(cid, mid)| (*cid, MessageId { message_id: *mid }))
                .collect();

            Ok(ret)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    /// Record new message id, so we can later see it returned
    /// from `order_msg_ids`
    pub async fn add_msg_id(
        &mut self,
        oid: OrderId,
        cid: ChatId,
        mid: MessageId,
    ) -> Result<(), Error> {
        let mid: i32 = mid.message_id;
        let db = self.db.clone();
        spawn_blocking(move || -> Result<(), Error> {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            let order_msgs =
                if let Some(order_msgs) = db.order_msgs.get_mut(&oid) {
                    order_msgs
                } else {
                    db.order_msgs.insert(oid, BTreeSet::new());
                    db.order_msgs.get_mut(&oid).unwrap()
                };
            order_msgs.insert((cid, mid));
            Ok(())
        }).await.map_err(|e| format!("{e:?}").into()).flatten()?;
        Ok(())
    }
}

#[derive(Debug)]
struct InnerDb {
    max_id: OrderId,
    /// list of known public chats and their members
    public_chats: Vec<PublicChat>,
    users: BTreeMap<UserId, User>,
    /// Messages sent for order, so we can remove or edit them
    pub order_msgs: BTreeMap<OrderId, BTreeSet<(ChatId, i32)>>,
}

impl Default for InnerDb {
    fn default() -> Self {
        InnerDb {
            public_chats: Vec::new(),
            users:        BTreeMap::new(),
            max_id:       OrderId(0),
            order_msgs:   BTreeMap::new(),
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
        order: &mut Order
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
        pub_chat.orders.push(order.clone());
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
            .filter(|o| o.customer.id == uid)
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
            return Err(ActionError::OrderNotFound(order_id))
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
        user: User,
        pub_chat_id: ChatId,
        action: &Action,
    ) -> Result<(order::Status, Option<Order>), ActionError> {
        let uid = user.id;
        log::info!("db.perform_action uid = {uid} pub_chat_id = {pub_chat_id}");
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
            let prev_status = order.perform_action(user, action)?;
            Ok((prev_status, Some(order.clone())))
        }
    }

    pub fn get_user(&self, uid: UserId) -> Option<&User> {
        self.users.get(&uid)
    }

    fn add_members<I: Iterator<Item=UserId>>(
        &mut self,
        cid: ChatId,
        uids: I,
    ) {
        let chat = self.public_chats.iter_mut().find(|c| c.chat.id == cid);
        if chat.is_none() {
            log::warn!("add_members:
Weird that we couldn't find pub chat {cid}");
            return;
        }
        let chat = chat.unwrap();
        for uid in uids {
            chat.add_user(uid);
        }
    }

    fn remove_member(&mut self, cid: ChatId, uid: UserId) {
        let chat = self.public_chats.iter_mut().find(|c| c.chat.id == cid);
        if chat.is_none() {
            log::warn!("remove_member:
Weird that we couldn't find pub chat {cid}");
            return;
        }
        let chat = chat.unwrap();
        chat.remove_user(uid);
    }

    pub fn update_user(&mut self, user: User) {
        if let Some(u) = self.users.get_mut(&user.id) {
            *u = user.clone();
        } else {
            self.users.insert(user.id, user);
        }
    }

    fn update_chat(&mut self, chat: Chat) -> Result<(), Error> {
        match chat.kind {
            ChatKind::Public(_) => {
                let c =
                    self.public_chats.iter_mut()
                        .find(|c| c.chat.id == chat.id);
                if let Some(c) = c {
                    // Update chat
                    c.chat = chat;
                } else {
                    // chat doesn't exist yet, so save it
                    let chat = PublicChat::new(chat);
                    self.public_chats.push(chat);
                }

            },
            ChatKind::Private(_) => {
                // Nothing to do here
                // TODO return error
                let cid: ChatId = chat.id;
                return Err(format!(
                        "chat {cid} is private, cannot update it").into())
            },
        }
        Ok(())
    }

    pub fn debug_stats(&self) -> String {
        let max_id = self.max_id;
        let num_public_chats = self.public_chats.len();
        let num_users = self.users.len();

        let mut users = String::new();
        for u in self.users.values() {
            let name = format!(
                "({}) @{} {}, ",
                u.id,
                u.username.clone().unwrap_or_else(|| "<noname>".to_string()),
                u.first_name);
            users.push_str(name.as_ref());
        }

        let mut pub_chats = String::new();
        let mut num_orders = 0;
        for pc in self.public_chats.iter() {
            let name = pc.chat.title().unwrap_or("<noname>");
            let pcid = pc.chat.id;
            let num_users = pc.members.len().to_string();
            let mut users = String::new();
            num_orders += pc.orders.len();
            for uid in pc.members.iter() {
                let u = self.users.get(uid).unwrap();
                let s = format!("  ({uid}) {}, ", u.username.clone().unwrap_or_else(|| "<noname>".to_string()));
                users.push_str(&s);
                users.push('\n');
            }
            let num_orders = pc.orders.len();
            let s = format!("{name} ({pcid}); {num_users} users, {num_orders} orders, users: \n{users} ");
            pub_chats.push_str(&s);
        }
        let s = format!("\
Data:
max_id =       {max_id}
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

