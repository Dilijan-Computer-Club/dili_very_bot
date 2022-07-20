use std::fmt;
use teloxide::{ prelude::*, types::User };

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
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord,
         Serialize, Deserialize)]
#[repr(transparent)]
pub struct OrderId(pub u64);

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
// Order
//
// We probably want this:
// - name
// - price
// - fee / markup
// - contact --- tg user
// - note / description

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    /// Id of this order, None if not persisted in the database
    pub id: Option<OrderId>,

    pub name: String,

    /// Original description message
    ///
    /// We probably don't need it, we just need the text, but whatever
    pub description_text: String,

    /// Roughly how much it is
    pub price_in_drams: u64,

    /// How much extra the customer is willing to pay
    pub markup_in_drams: u64,

    /// When it was created (not published)
    pub created_at: DateTime,

    /// When it was published
    pub published_at: Option<DateTime>,

    /// Who created this order
    pub customer: User,

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
    /// Returns true if is assigned and not completed yet
    pub fn is_active_assignment(&self) -> bool {
        match self.status() {
            Status::Assigned          => true,
            Status::MarkedAsDelivered => true,
            _ => false,
        }
    }

    fn role(&self, uid: UserId) -> Role {
        if self.customer.id == uid {
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
            Status::Assigned => &[
                ActionKind::Unassign,
                ActionKind::MarkAsDelivered,
                ActionKind::ConfirmDelivery
            ],
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
        user: User,
        action: &Action
    ) -> Result<Status, ActionError> {
        let uid = user.id;
        if ! self.is_action_permitted(uid, action) {
            return Err(ActionError::NotPermitted)
        }

        let prev_status = self.status();

        match action.kind {
            ActionKind::Publish => {
                self.canceled_at = None;
                self.published_at = Some(Offset::now());
            },
            ActionKind::Cancel => {
                self.canceled_at = Some(Offset::now());
            },
            ActionKind::AssignToMe => {
                self.assigned = Some((Offset::now(), uid, Some(user)));
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

    fn mk_customer() -> teloxide::types::User {
        teloxide::types::User {
            id: teloxide::types::UserId(1),
            first_name: "firstname".into(),
            last_name: None,
            username: None,
            is_bot: false,
            language_code: None,
        }
    }

    #[test]
    fn test_order_status_changes() {
        let publisher = User {
            id: UserId(1),
            username: Some("publisher".to_string()),
            first_name: "Publisher".to_string(),
            last_name: Some("test".to_string()),
            is_bot: false,
            language_code: Some("en".to_string()),
        };
        let assignee = User {
            id: UserId(2),
            username: Some("assignee".to_string()),
            first_name: "Assignee".to_string(),
            last_name: Some("test".to_string()),
            is_bot: false,
            language_code: Some("am".to_string()),
        };

        let oid = OrderId(1);

        let customer = mk_customer();
        let order = Order {
            id: Some(oid),
            name: "ordername".to_string(),
            price_in_drams: 0,
            markup_in_drams: 0,
            description_text: "order description".to_string(),
            created_at: chrono::offset::Utc::now(),
            canceled_at: None,
            delivered: None,
            published_at: None,
            customer,
            assigned: None,
            delivery_confirmed_at: None,
        };

        let act = |order: &mut Order, action: ActionKind, actor: User, expected_status: Status| {
            order.perform_action(actor, &Action {
                kind: action, order_id: oid,
            }).unwrap();
            assert_eq!(expected_status, order.status());
        };

        // happy path
        {
            let mut order = order.clone();

            act(&mut order, ActionKind::Publish,         publisher.clone(), Status::Published);
            act(&mut order, ActionKind::AssignToMe,      assignee.clone(),  Status::Assigned);
            act(&mut order, ActionKind::MarkAsDelivered, assignee.clone(),  Status::MarkedAsDelivered);
            act(&mut order, ActionKind::ConfirmDelivery, publisher.clone(), Status::DeliveryConfirmed);
        }

        // happy path, but confirmed without marking as delivered
        {
            let mut order = order;

            act(&mut order, ActionKind::Publish,         publisher.clone(), Status::Published);
            act(&mut order, ActionKind::AssignToMe,      assignee,          Status::Assigned);
            act(&mut order, ActionKind::ConfirmDelivery, publisher,         Status::DeliveryConfirmed);
        }
    }
}
