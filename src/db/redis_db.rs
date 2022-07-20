#![cfg(feature = "redis_db")]

use teloxide::prelude::*;
use teloxide::types::{User, Chat, MessageId};
use redis;
use crate::error::Error;
use crate::order::{self, Order, OrderId, Action, ActionKind,
                   Status, ActionError};
use serde_json;

fn to_err(e: redis::RedisError) -> Error {
    format!("Redis error: {e:?}").into()
}

/// Structure:
///   num_orders            u64
///   users                 Set<UserId>
///   user:id               SerializedData
///   uesr:id:orders        Set<(ChatId, OrderId)>
///   pub_chats             Set<ChatId>
///   pub_chat:id           SerializedData
///   pub_chat:id:name      String
///   pub_chat:id:members   Set<UserId>
///   user:id:public_chats  Set<ChatId>
///   pub_chat:id:orders    Set<OrderId>
///   pub_chat:id:order:id  SerializedData
///   order_msgs:id         Set<(ChatId, MessageId)>
#[derive(Clone)]
pub struct Db {
    c: redis::aio::ConnectionManager,
}

use crate::REDIS_URL;

impl Db {
    pub async fn new() -> Result<Self, Error> {
        let client = redis::Client::open(REDIS_URL)
            .map_err(to_err)?;
        let connection = client.get_tokio_connection_manager()
            .await.map_err(to_err)?;

        let db = Db { c: connection };

        Ok(db)
    }

    pub async fn user_public_chats(
        &mut self,
        uid: UserId,
    ) -> Result<Vec<(ChatId, String)>, Error> {
        log::debug!("user_public_chats");

        let pub_chats: Vec<i64> =
            redis::Cmd::smembers(user_pub_chats_key(uid))
            .query_async(&mut self.c).await?;
        // it's an error to query nothing
        if pub_chats.is_empty() {
            return Ok(Vec::new())
        }

        let keys: Vec<String> =
            pub_chats.iter().map(|pc| pub_chat_name_key(ChatId(*pc)))
            .collect();
        log::debug!("pub chat keys {keys:?}");
        let mut names: Vec<String> = Vec::new();
        if keys.len() == 1 {
            let name: String = redis::Cmd::get(&keys[0])
                .query_async(&mut self.c).await?;
            names.push(name)
        } else {
            names = redis::Cmd::get(keys)
                .query_async(&mut self.c).await?;
        }
        if names.len() != pub_chats.len() {
            return Err(format!("wrong number of received chat names: {} \
insteadd of {}", names.len(), pub_chats.len()).into());
        }

        Ok(pub_chats.into_iter().map(ChatId).zip(names.into_iter()).collect())
    }

    /// Returns new order's `OrderId`
    /// Also updates the order itself
    pub async fn add_order(
        &mut self,
        pcid: ChatId,
        order: &mut Order
    ) -> Result<OrderId, Error> {
        log::debug!("add_order {pcid} {:?}", order.id);

        // It's not a data race, is it?
        let oid: u64 = redis::Cmd::incr(num_orders_key(), 1)
            .query_async(&mut self.c).await?;
        let oid = OrderId(oid);
        order.id = Some(oid);

        redis::pipe()
            .atomic()
            .set(pub_chat_order_key(pcid, oid), serde_json::to_vec(order)?)
            .sadd(pub_chat_orders_key(pcid), oid.0)
            .sadd(user_orders_key(order.customer.id), oid.0)
            .query_async(&mut self.c).await?;
        Ok(oid)
    }

    /// Returns some debugging info
    pub async fn debug_stats(&mut self) -> Result<String, Error> {
        log::debug!("debug_stats");

        let mut ret = String::new();
        let num_orders: u64 =
            redis::Cmd::get(num_orders_key())
            .query_async(&mut self.c).await.map_err(to_err)?;
        ret.push_str(format!("num orders = {num_orders}\n").as_ref());

        let pub_chats: Vec<i64> =
            redis::Cmd::smembers(pub_chats_key())
            .query_async(&mut self.c).await.map_err(to_err)?;
        ret.push_str(format!("pub chats ({}) = {pub_chats:?}\n",
                             pub_chats.len()).as_ref());

        let uids: Vec<i64> = redis::Cmd::smembers(users_key())
            .query_async(&mut self.c).await.map_err(to_err)?;
        ret.push_str(format!("uids ({}) = {uids:?}\n",
                             uids.len()).as_ref());

        log::debug!("{}", ret);
        Ok(ret)
    }

    /// Returns Ok(None) if there is no user
    pub async fn get_user(
        &mut self,
        uid: UserId
    ) -> Result<Option<User>, Error> {
        log::debug!("get_user {uid}");

        let bytes: Vec<u8> = redis::Cmd::get(user_key(uid))
            .query_async(&mut self.c)
            .await.map_err(to_err)?;
        if bytes.is_empty() {
            return Ok(None)
        }
        let user: User = serde_json::from_slice(&bytes)?;
        Ok(Some(user))
    }

    /// Updates the user in the database
    pub async fn update_user(
        &mut self,
        user: User,
    ) -> Result<(), Error> {
        log::debug!("update_user {:?}", user.id);

        redis::pipe()
            .atomic()
            .set(user_key(user.id), serde_json::to_vec(&user)?)
            .sadd(users_key(), user.id.0)
            .query_async(&mut self.c).await?;
        Ok(())
    }

    /// Add new member to public chat
    pub async fn add_members(
        &mut self,
        cid: ChatId,
        uids: Vec<UserId>, // can it be a slice instead?
    ) -> Result<(), Error> {
        log::debug!("add_members {cid} {uids:?}");

        if cid.is_user() {
            log::warn!("add_members trying add members \
to private chat {cid} {uids:?}");
            return Ok(())
        }

        let uids: Vec<u64> = uids.into_iter().map(|u| u.0).collect();
        let mut pipe = redis::pipe();
        for uid in uids.iter() {
            pipe.sadd(user_pub_chats_key(UserId(*uid)), cid.0);
        }
        pipe.sadd(pub_chat_members_key(cid), uids);
        pipe.query_async(&mut self.c).await.map_err(to_err)
    }

    /// Remove user from chat
    pub async fn remove_chat_membership(
        &mut self,
        cid: ChatId,
        uid: UserId,
    ) -> Result<(), Error> {
        log::debug!("remove_chat_membership {cid} {uid}");

        redis::Cmd::srem(pub_chat_members_key(cid), uid.0)
            .query_async(&mut self.c).await.map_err(to_err)
    }

    /// Return all orders in a public chat
    async fn pub_chat_orders(
        &mut self,
        pcid: ChatId,
    ) -> Result<Vec<Order>, Error> {
        log::debug!("pub_chat_orders {pcid}");

        let oids: Vec<u64> =
            redis::Cmd::smembers(pub_chat_orders_key(pcid))
            .query_async(&mut self.c).await.map_err(to_err)?;
        let order_keys: Vec<String> = oids.into_iter()
            .map(|oid| pub_chat_order_key(pcid, OrderId(oid)))
            .collect();
        // Redis doesn't allow to query for no keys,
        // so it's not just an optimization
        if order_keys.is_empty() {
            return Ok(Vec::new())
        }

        let bin_orders: Vec<Vec<u8>> =
            if order_keys.len() == 1 {
                vec![redis::Cmd::get(order_keys[0].clone())
                    .query_async(&mut self.c).await.map_err(to_err)?]
            } else {
                redis::Cmd::get(order_keys)
                    .query_async(&mut self.c).await.map_err(to_err)?
            };

        let mut orders: Vec<Order> = Vec::with_capacity(bin_orders.len());
        for bin_o in bin_orders.into_iter() {
            orders.push(serde_json::from_slice(&bin_o)?);
        }
        Ok(orders)
    }

    /// Return orders in the chat, filtered by `status`
    pub async fn orders_by_status(
        &mut self,
        pcid: ChatId,
        status: Status,
    ) -> Result<Vec<Order>, Error> {
        log::debug!("orders_by_status {pcid} {status:?}");

        // TODO optimize it later
        let orders = self.pub_chat_orders(pcid).await?;
        let orders = orders.into_iter()
            .filter(|o| o.status() == status).collect();

        Ok(orders)
    }

    /// Return orders in `pcid` assigned to `uid`
    pub async fn active_assignments_to(
        &mut self,
        pcid: ChatId,
        uid: UserId
    ) -> Result<Vec<Order>, Error> {
        log::debug!("active_assignments_to {pcid} {uid:?}");
        // TODO optimize later

        let orders = self.pub_chat_orders(pcid).await?;
        Ok(orders.into_iter()
           .filter(|o| o.is_active_assignment())
           .filter(|o| {
            if let Some(assigned) = &o.assigned {
                assigned.1 == uid
            } else {
                false
            }
        }).collect())
    }

    /// Return orders in `pcid` that are assigned to `uid`
    pub async fn orders_submitted_by_user(
        &mut self,
        pcid: ChatId,
        uid: UserId
    ) -> Result<Vec<Order>, Error> {
        log::debug!("ordes_submitted_by_user {pcid} {uid:?}");

        let oids: Vec<u64> =
            redis::Cmd::sinter(
                &[user_orders_key(uid), pub_chat_orders_key(pcid)])
            .query_async(&mut self.c).await.map_err(to_err)?;
        let keys: Vec<String> = oids.into_iter()
            .map(|oid| pub_chat_order_key(pcid, OrderId(oid)))
            .collect();

        if keys.is_empty() {
            return Ok(Vec::new());
        }

        if keys.len() == 1 {
            let order: Vec<u8> = redis::Cmd::get(&keys[0])
                .query_async(&mut self.c).await.map_err(to_err)?;
            let order: Order = serde_json::from_slice(&order)?;
            return Ok(vec![order]);
        }

        let num_keys = keys.len();
        let mut pipe = redis::pipe();
        for key in keys.into_iter() {
            pipe.get(key);
        }
        let bin_orders: Vec<Vec<u8>> =
            pipe.query_async(&mut self.c).await.map_err(to_err)?;
        let mut orders = Vec::with_capacity(num_keys);
        for b in bin_orders.into_iter() {
            orders.push(serde_json::from_slice(&b)?);
        }
        Ok(orders)
    }

    /// Performs the action and returns previous state and the Order
    /// If the order is deleted then the returned order is None
    pub async fn perform_action(
        &mut self,
        user: User,
        pcid: ChatId,
        action: Action,
    ) -> Result<(order::Status, Option<Order>), ActionError> {
        let uid = user.id;
        log::debug!("perform_action {uid} {pcid} {action:?}");

        let order: Option<Order> =
            self.get_order(pcid, action.order_id)
            .await.map_err(|_| ActionError::Other)?;
        if order.is_none() {
            return Err(ActionError::OrderNotFound(action.order_id));
        }
        let order = order.unwrap();
        if ! order.is_action_permitted(uid, &action) {
            return Err(ActionError::NotPermitted)
        }

        if action.kind == ActionKind::Delete {
            let res = self.delete_order_unchecked(
                pcid, order.customer.id, action.order_id).await;
            if let Err(e) = res {
                log::warn!("perform_action {uid} {pcid} : {e:?}");
                return Err(ActionError::Other);
            }
            Ok((order.status(), None))
        } else {
            let mut order = order;
            let prev_status = order.perform_action(user, &action)?;
                log::warn!("perform_action {uid} {pcid} : {prev_status} => {}", order.status());
            let res = self.update_order(pcid, &order)
                .await;
            if let Err(e) = res {
                log::warn!("perform_action {uid} {pcid} : {e:?}");
                return Err(ActionError::Other);
            }
            Ok((prev_status, Some(order)))
        }
    }

    /// Deletes the order without checking permissions
    async fn delete_order_unchecked(
        &mut self,
        pcid: ChatId,
        uid: UserId,
        oid: OrderId,
    ) -> Result<(), Error> {
        redis::pipe()
            .atomic()
            .del(pub_chat_order_key(pcid, oid))
            .srem(pub_chat_orders_key(pcid), oid.0)
            .srem(user_orders_key(uid), oid.0)
            .del(order_msgs_key(oid))
            .query_async(&mut self.c).await.map_err(to_err)?;
        Ok(())
    }

    /// Get data of order that's in `pcid`
    async fn get_order(
        &mut self,
        pcid: ChatId,
        oid: OrderId,
    ) -> Result<Option<Order>, Error> {
        let data: Option<Vec<u8>> = redis::Cmd::get(pub_chat_order_key(pcid, oid))
            .query_async(&mut self.c).await.map_err(to_err)?;
        if data.is_none() {
            return Ok(None)
        }
        let data = data.unwrap();
        let order = serde_json::from_slice(&data)?;

        Ok(Some(order))
    }

    /// Update order data in the database
    async fn update_order(
        &mut self,
        pcid: ChatId,
        order: &Order,
    ) -> Result<(), Error> {
        let oid = order.id;
        if oid.is_none() {
            return Err("order has no id".into())
        }
        let oid = oid.unwrap();
        let data: Vec<u8> = serde_json::to_vec(order)?;
        redis::Cmd::set(pub_chat_order_key(pcid, oid), data)
            .query_async(&mut self.c).await.map_err(to_err)?;
        Ok(())
    }

    /// Update chat data in the database
    pub async fn update_chat(
        &mut self,
        chat: Chat,
    ) -> Result<(), Error> {
        let title = chat.title().unwrap_or("");
        log::debug!("update_chat \"{title}\" {chat:?}");

        redis::pipe()
            .sadd(pub_chats_key(), chat.id.0)
            .set(pub_chat_key(chat.id), serde_json::to_vec(&chat)?)
            .set(pub_chat_name_key(chat.id), title)
            .query_async(&mut self.c).await.map_err(to_err)
    }

    /// Get which messages we've sent that contain this order
    ///
    /// Returns pairs of (chat_id, order_id) because we need `chat_id` to
    /// change or delete these messages
    pub async fn order_msg_ids(
        &mut self,
        oid: OrderId,
    ) -> Result<Vec<(ChatId, MessageId)>, Error> {
        log::debug!("order_msg_id {oid:?}");
        let data_items: Vec<Vec<u8>> =
            redis::Cmd::smembers(order_msgs_key(oid))
            .query_async(&mut self.c).await?;

        let mut cid_mids: Vec<(ChatId, MessageId)> =
            Vec::with_capacity(data_items.len());
        for data in data_items.into_iter() {
            let (cid, mid) = serde_json::from_slice(&data)?;
            cid_mids.push((cid, MessageId { message_id: mid }));
        }

        Ok(cid_mids)
    }

    /// Record new message id, so we can later see it returned
    /// from `order_msg_ids`
    pub async fn add_msg_id(
        &mut self,
        oid: OrderId,
        cid: ChatId,
        mid: MessageId,
    ) -> Result<(), Error> {
        let pair = (cid, mid.message_id);
        let data: Vec<u8> =
            serde_json::to_vec(&pair)?;
        redis::Cmd::sadd(order_msgs_key(oid), data)
            .query_async(&mut self.c).await.map_err(to_err)?;

        Ok(())
    }
}

const PREFIX: &str = "dili";

// unfortunately I don't think it's possible to concat strings in const fn
const fn users_key() -> &'static str {
    "dili_users"
}

// unfortunately I don't think it's possible to concat strings in const fn
const fn pub_chats_key() -> &'static str {
    "dili_pub_chats"
}

// unfortunately I don't think it's possible to concat strings in const fn
const fn num_orders_key() -> &'static str {
    "dili_num_orders"
}

fn key(k: &str) -> String {
    let p = PREFIX;
    format!("{p}_{k}")
}

fn user_key(uid: UserId) -> String {
    key(format!("user:{uid}").as_ref())
}

fn user_orders_key(uid: UserId) -> String {
    user_key(uid) + ":orders"
}

fn user_pub_chats_key(uid: UserId) -> String {
    user_key(uid) + ":pub_chats"
}

fn pub_chat_key(pc: ChatId) -> String {
    key(format!("pub_chat:{pc}").as_ref())
}

fn pub_chat_name_key(pc: ChatId) -> String {
    pub_chat_key(pc) + ":name"
}

fn pub_chat_members_key(pc: ChatId) -> String {
    pub_chat_key(pc) + ":members"
}

fn pub_chat_orders_key(pc: ChatId) -> String {
    pub_chat_key(pc) + ":orders"
}

fn pub_chat_order_key(pc: ChatId, oid: OrderId) -> String {
    let k = pub_chat_key(pc);
    format!("{k}:order:{oid}")
}

fn order_msgs_key(oid: OrderId) -> String {
    key(&format!("order_msgs:{oid}"))
}
