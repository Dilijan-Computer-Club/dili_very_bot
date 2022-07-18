use std::fmt;
use teloxide::{ prelude::*, types::User };
use crate::tg_msg::TgMsg;
use crate::error::Error;
use crate::urgency::Urgency;

mod action_kind;
mod action;
mod status;
mod role;
mod action_error;
pub use status::Status;
pub use role::Role;
pub use action::Action;
pub use action_kind::ActionKind;
pub use action_error::ActionError;
use crate::utils::dumb_intersection;
use crate::Offset;
use crate::DateTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct OrderId(pub u64);

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Order {
    /// Id of this order, None if not persisted in the database
    pub id: Option<OrderId>,

    /// Original description message
    pub desc_msg: TgMsg,

    /// Roughly how much it is
    pub price_in_drams: u64,

    /// How soon we need it
    pub urgency: Urgency,

    /// When it was created (not published)
    pub created_at: DateTime,

    /// When it was published
    pub published_at: Option<DateTime>,

    /// Who created this order
    pub from: User,

    /// When the user was assigned, id of the user that was assigned,
    /// and the user itself it we have it (we might not)
    pub assigned: Option<(DateTime, UserId, Option<User>)>,

    /// When and by whom it was delivered, if it was
    pub delivered: Option<(UserId, Option<User>, DateTime)>,

    /// When the delivery was confirmed, None if it's not
    pub delivery_confirmed_at: Option<DateTime>,

    /// When it was canceled, if it was
    pub canceled_at: Option<DateTime>,
}

impl Order {
    // TODO should replace this with something else
    pub fn from_tg_msg(tg_msg: &Message) -> Result<Order, Error> {
        let msg = TgMsg::from_tg_msg(tg_msg)?;
        let from = match tg_msg.from() {
            Some(user) => user,
            None => return Err("No 'from' in message".into()),
        };

        Ok(Order {
            id: None,
            created_at: chrono::offset::Utc::now(),
            price_in_drams: 0,
            canceled_at: None,
            delivered: None,
            published_at: None,
            urgency: Urgency::Whenever,
            desc_msg: msg,
            from: from.clone(),
            assigned: None,
            delivery_confirmed_at: None,
        })
    }

    /// Returns true if is assigned and not completed yet
    pub fn is_active_assignment(&self) -> bool {
        match self.status() {
            Status::Assigned          => true,
            Status::MarkedAsDelivered => true,
            _ => false,
        }
    }

    fn role(&self, uid: UserId) -> Role {
        if self.from.id == uid {
            return Role::Owner;
        }

        if let Some((_when, assignee_id, _user)) = &self.assigned {
            if assignee_id == &uid {
                return Role::Assignee;
            }
        }

        Role::UnrelatedUser
    }

    pub const fn status(&self) -> Status {
        if self.canceled_at.is_some() { return Status::Unpublished }
        if self.delivery_confirmed_at.is_some() {
            return Status::DeliveryConfirmed }
        if self.delivered.is_some() { return Status::MarkedAsDelivered }
        if self.assigned.is_some() { return Status::Assigned }
        if self.published_at.is_some() { return Status::Published }

        Status::Unpublished
    }

    /// Actions that are available to order in its current state
    pub const fn available_actions(&self) -> &'static [ActionKind] {
        match self.status() {
            Status::Unpublished =>
                &[ActionKind::Publish, ActionKind::Delete],
            Status::Published =>
                &[ActionKind::AssignToMe, ActionKind::Cancel],
            Status::Assigned =>
                &[ActionKind::Unassign, ActionKind::MarkAsDelivered,
                  ActionKind::ConfirmDelivery],
            Status::MarkedAsDelivered =>
                &[ActionKind::ConfirmDelivery],
            Status::DeliveryConfirmed =>
                &[ActionKind::Delete],
        }
    }

    pub fn user_actions(
        &self,
        actor: UserId,
    ) -> Vec<ActionKind> {
        let role = self.role(actor);
        let available_actions = self.available_actions();
        let allowed_actions = role.allowed_actions();

        dumb_intersection(allowed_actions, available_actions)
    }

    pub fn public_actions(&self) -> Vec<ActionKind> {
        let available_actions = self.available_actions();
        let allowed_actions = Role::UnrelatedUser.allowed_actions();

        dumb_intersection(allowed_actions, available_actions)
    }

    pub fn is_action_permitted(&self, uid: UserId, action: &Action) -> bool {
        let allowed = self.user_actions(uid);
        allowed.into_iter().any(|a| a == action.kind)
    }

    /// Performs `action` and returns previous status
    ///
    /// Note: shouldn't be called with `Delete` action, which should
    /// be handled by the database instead
    pub fn perform_action(
        &mut self,
        uid: UserId,
        action: &Action
    ) -> Result<Status, ActionError> {
        if ! self.is_action_permitted(uid, action) {
            return Err(ActionError::NotPermitted)
        }

        let prev_status = self.status();

        match action.kind {
            ActionKind::Publish => {
                self.published_at = Some(Offset::now());
            },
            ActionKind::Cancel => {
                self.canceled_at = Some(Offset::now());
            },
            ActionKind::AssignToMe => {
                self.assigned = Some((Offset::now(), uid, None));
            },
            ActionKind::Unassign => {
                self.assigned = None;
            },
            ActionKind::MarkAsDelivered => {
                self.delivered = Some((uid, None, Offset::now()));
            },
            ActionKind::ConfirmDelivery => {
                self.delivery_confirmed_at = Some(Offset::now())
            },
            ActionKind::Delete => {
                panic!("should be handled by the database")
            },
        }

        Ok(prev_status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_msg() -> (TgMsg, teloxide::types::User) {
        let msg = TgMsg {
            chat_id: ChatId(1),
            message_id: 2,
            text: "msg text".to_string(),
        };
        let from = teloxide::types::User {
            id: teloxide::types::UserId(1),
            first_name: "firstname".into(),
            last_name: None,
            username: None,
            is_bot: false,
            language_code: None,
        };

        (msg, from)
    }

    #[test]
    fn test_order_status_changes() {
        let publisher = UserId(1);
        let assignee = UserId(2);
        let oid = OrderId(1);

        let (msg, from) = mk_msg();
        let order = Order {
            id: Some(oid),
            created_at: chrono::offset::Utc::now(),
            price_in_drams: 0,
            canceled_at: None,
            delivered: None,
            published_at: None,
            urgency: Urgency::Whenever,
            desc_msg: msg,
            from: from.clone(),
            assigned: None,
            delivery_confirmed_at: None,
        };

        let act = |order: &mut Order, action: ActionKind, actor: UserId, expected_status: Status| {
            order.perform_action(actor, &Action {
                kind: action, order_id: oid,
            }).unwrap();
            assert_eq!(expected_status, order.status());
        };

        // happy path
        {
            let mut order = order.clone();

            act(&mut order, ActionKind::Publish,         publisher, Status::Published);
            act(&mut order, ActionKind::AssignToMe,      assignee,  Status::Assigned);
            act(&mut order, ActionKind::MarkAsDelivered, assignee,  Status::MarkedAsDelivered);
            act(&mut order, ActionKind::ConfirmDelivery, publisher, Status::DeliveryConfirmed);
        }

        // happy path, but confirmed without marking as delivered
        {
            let mut order = order.clone();

            act(&mut order, ActionKind::Publish,         publisher, Status::Published);
            act(&mut order, ActionKind::AssignToMe,      assignee,  Status::Assigned);
            act(&mut order, ActionKind::ConfirmDelivery, publisher, Status::DeliveryConfirmed);
        }
    }
}
