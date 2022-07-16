use tokio::task::spawn_blocking;
use teloxide::prelude::*;
use teloxide::types::{User};
use std::sync::{Arc, RwLock};
use std::collections::BTreeMap;
use crate::error::Error;

use crate::order::{self, Order, OrderId, SpecificAction, Status};
use crate::order_action::OrderAction;


/// Wrapper for InnerDb that is Send, Sync, and async
#[derive(Clone)]
pub struct Db {
    db: Arc<RwLock<InnerDb>>,
}

impl Db {
    pub fn new() -> Self {
        Db { db: Arc::new(RwLock::new(InnerDb::default())) }
    }

    pub async fn add_order(&mut self, order: &Order) -> Result<OrderId, Error> {
        let db = self.db.clone();
        let order = order.clone();
        spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.add_order(order)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn orders_by_status(
        &self,
        status: Status,
    ) -> Result<Vec<Order>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            db.orders_by_status(status)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn active_assignments_to(
        &self,
        uid: UserId
    ) -> Result<Vec<Order>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            db.active_assignments_to(uid)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    pub async fn orders_submitted_by_user(
        &self,
        uid: UserId
    ) -> Result<Vec<Order>, Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let db = db.read().map_err(|e| format!("Rlock: {e:?}"))?;
            db.orders_submitted_by_user(uid)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }

    /// Performs the action and returns previous state and the Order
    /// If the order is deleted then the returned order is None
    pub async fn perform_action(
        &mut self,
        action: SpecificAction,
    ) -> Result<(order::Status, Option<Order>), Error> {
        let db = self.db.clone();
        spawn_blocking(move || {
            let mut db = db.write().map_err(|e| format!("lock: {e:?}"))?;
            db.perform_action(&action)
        }).await.map_err(|e| format!("{e:?}").into()).flatten()
    }
}

#[derive(Debug)]
struct InnerDb {
    max_id: OrderId,
    orders: Vec<Order>,
    users: BTreeMap<UserId, User>,
}

impl Default for InnerDb {
    fn default() -> Self {
        InnerDb {
            users: BTreeMap::new(),
            max_id: OrderId(0),
            orders: Vec::new()
        }
    }
}

impl InnerDb {
    fn next_id(&mut self) -> OrderId {
        self.max_id.0 += 1;
        self.max_id
    }

    pub fn add_order(&mut self, mut order: Order) -> Result<OrderId, Error> {
        let new_id = self.next_id();

        log::info!("Added order {order:?} new id = {}", new_id.0);

        order.id = Some(new_id);
        self.orders.push(order);
        Ok(new_id)
    }

    pub fn orders_by_status(
        &self,
        status: Status,
    ) -> Result<Vec<Order>, Error> {
        log::info!("Listing orders of status {status}");

        Ok(self.orders.iter()
            .filter(|o| o.status() == status)
            .cloned()
            .collect())
    }

    pub fn orders_submitted_by_user(
        &self,
        uid: UserId
    ) -> Result<Vec<Order>, Error> {
        log::info!("Listing orders submitted by user {:?}", uid);
        Ok(self.orders.iter()
            .filter(|o| o.from.id == uid)
            .cloned()
            .collect())
    }

    pub fn active_assignments_to(
        &self,
        uid: UserId,
    ) -> Result<Vec<Order>, Error> {
        log::info!("Listing orders assigned to {:?}", uid);

        Ok(self.orders.iter()
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

    fn find_order(&mut self, order_id: OrderId) -> Option<&Order> {
        self.orders.iter().find(|o| o.id == Some(order_id) )
    }

    fn find_order_mut(&mut self, order_id: OrderId) -> Option<&mut Order> {
        self.orders.iter_mut().find(|o| o.id == Some(order_id) )
    }

    /// Deletes the order and returns its previous status
    fn delete_order(&mut self, order_id: OrderId) -> Result<order::Status, Error> {
        for (ii, order) in self.orders.iter().enumerate() {
            if order.id == Some(order_id) {
                let status = order.status();
                self.orders.remove(ii);
                return Ok(status);
            }
        }
        Err(format!("Could not find order {:?}", order_id).into())
    }

    /// performs the action, returns modified order if successful
    pub fn perform_action(
        &mut self,
        action: &SpecificAction,
    ) -> Result<(order::Status, Option<Order>), Error> {
        if action.action == OrderAction::Delete {
            let order = self.find_order(action.order_id)
                .ok_or_else(|| format!("Could not find order {:?}",
                                       action.order_id))?;
            if ! order.is_action_permitted(action) {
                return Err(format!("Action {} is not permitted",
                                   action.action.human_name()).into())
            }

            let status = self.delete_order(action.order_id)?;
            Ok((status, None))
        } else {
            let order = self.find_order_mut(action.order_id)
                .ok_or_else(|| format!("Could not find order {:?}",
                                       action.order_id))?;
            let prev_status = order.perform_action(action)?;
            Ok((prev_status, Some(order.clone())))
        }
    }
}
