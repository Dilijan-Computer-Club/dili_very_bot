use teloxide::prelude::*;
use teloxide::types::User;

use std::sync::{Arc, RwLock};

use std::sync::atomic::AtomicU64;

use crate::order::Order;
use crate::tg_msg::TgMsg;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct Db {
    orders: Arc<RwLock<Vec<Order>>>,
}

impl Db {
    pub fn new() -> Db {
        Db { orders: Arc::new(RwLock::new(Vec::new())) }
    }

    // TODO should be async
    pub fn add_order(&mut self, order: &Order) -> Result<(), Error> {
        log::info!("Added order {order:?}");
        let order = order.clone();
        // TODO return if errors
        let mut orders = self.orders.write().unwrap();
        orders.push(order);
        Ok(())
    }

    // TODO should be async
    pub fn list_all_orders(&self) -> Result<Vec<Order>, Error> {
        log::info!("Listing orders: {:?}", self.orders);
        let orders = self.orders.read().unwrap();
        Ok(orders.clone())
    }

    // TODO should be async
    pub fn orders_submitted_by_user(&self, uid: UserId) -> Result<Vec<Order>, Error> {
        log::info!("Listing orders submitted by user {:?}: {:?}",
                   uid, self.orders);
        let orders = self.orders.read().unwrap();
        let res = orders.iter()
            .filter(|o| o.from.id == uid)
            .cloned()
            .collect();

        Ok(res)
    }
}
