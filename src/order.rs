use std::fmt;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, User}
};
use crate::tg_msg::TgMsg;
use crate::error::Error;
use crate::order_action::OrderAction;
use crate::urgency::Urgency;

type Offset = chrono::offset::Utc;
type DateTime = chrono::DateTime<Offset>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct OrderId(pub u64);

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Role {
    Owner,
    Assignee,
    UnrelatedUser,
}

impl Role {
    const fn allowed_actions(self) -> &'static [OrderAction] {
        match self {
            Role::Owner =>
                &[OrderAction::Publish,
                  OrderAction::Cancel,
                  OrderAction::ConfirmDelivery,
                  OrderAction::Delete],
            Role::Assignee =>
                &[OrderAction::Unassign,
                OrderAction::MarkAsDelivered],
            Role::UnrelatedUser => &[OrderAction::AssignToMe],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Unpublished,
    Published,
    Assigned,
    MarkedAsDelivered,
    DeliveryConfirmed,
}

impl Status {
    pub const fn human_name(self) -> &'static str {
        match self {
            Status::Unpublished       => "Not published",
            Status::Published         => "Published",
            Status::Assigned          => "Assigned",
            Status::MarkedAsDelivered => "Marked as delivered",
            Status::DeliveryConfirmed => "Delivered",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.human_name())
    }
}


fn dumb_intersection<T: Clone + PartialEq>(aa: &[T], bb: &[T]) -> Vec<T> {
    let mut res = Vec::with_capacity(aa.len().max(bb.len()));
    for a in aa.iter() {
        for b in bb.iter() {
            if a == b { res.push(a.clone()) }
        }
    }
    res
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
            urgency: Urgency::Whatever,
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

    pub fn is_published(&self) -> bool {
        self.published_at.is_some()
    }

    pub fn description_text(&self) -> String {
        self.desc_msg.text.clone()
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
    pub const fn available_actions(&self) -> &'static [OrderAction] {
        match self.status() {
            Status::Unpublished =>
                &[OrderAction::Publish, OrderAction::Delete],
            Status::Published =>
                &[OrderAction::AssignToMe, OrderAction::Cancel],
            Status::Assigned =>
                &[OrderAction::Unassign, OrderAction::MarkAsDelivered,
                  OrderAction::ConfirmDelivery],
            Status::MarkedAsDelivered =>
                &[OrderAction::ConfirmDelivery],
            Status::DeliveryConfirmed =>
                &[OrderAction::Delete],
        }
    }

    pub fn user_actions(
        &self,
        actor: UserId,
    ) -> Vec<OrderAction> {
        let role = self.role(actor);
        let available_actions = self.available_actions();
        let allowed_actions = role.allowed_actions();

        dumb_intersection(allowed_actions, available_actions)
    }

    pub fn public_actions(&self) -> Vec<OrderAction> {
        let role = Role::UnrelatedUser;
        let available_actions = self.available_actions();
        let allowed_actions = role.allowed_actions();

        dumb_intersection(allowed_actions, available_actions)
    }

    /// Send a message that shows this order
    /// Arguments
    ///
    /// public:
    ///   If the message is for a public chat
    pub async fn send_message_for(
        &self,
        bot: &mut AutoSend<Bot>,
        uid: UserId,
        chat_id: ChatId,
        public: bool,
    ) -> Result<(), Error> {
        let bot = bot.parse_mode(teloxide::types::ParseMode::Html);
        let description = &self.desc_msg.text;
        let username = format_username(&self.from);
        let user_id = self.from.id;
        let order_id = self.id
            .ok_or("Could not make action for order without id")?;

        let actions =
            if public {
                self.public_actions()
            } else { 
                self.user_actions(uid)
            };

        let specific_actions: Vec<SpecificAction> =
            actions.into_iter()
            .map(|action| SpecificAction {
                actor: uid,
                action,
                order_id })
            .collect();
        let buttons = actions_keyboard_markup(&specific_actions);

        let status = self.status();
        let text = format!("\
<a href=\"tg://user?id={user_id}\">{username}</a>
{status}

{description}");
        bot.send_message(chat_id, text)
            .reply_markup(buttons).await?;
        Ok(())
    }

    pub fn is_action_permitted(&self, action: &SpecificAction) -> bool {
        let allowed = self.user_actions(action.actor);
        allowed.into_iter().any(|a| a == action.action)
    }

    /// Performs `action` and returns previous status
    ///
    /// Note: shouldn't be called with `Delete` action, which should
    /// be handled by the database instead
    pub fn perform_action(
        &mut self,
        action: &SpecificAction
    ) -> Result<Status, Error> {
        if ! self.is_action_permitted(action) {
            return Err(format!("Action {} is not permitted",
                               action.action.human_name()).into())
        }

        let prev_status = self.status();

        match action.action {
            OrderAction::Publish => {
                self.published_at = Some(Offset::now());
            },
            OrderAction::Cancel => {
                self.canceled_at = Some(Offset::now());
            },
            OrderAction::AssignToMe => {
                self.assigned = Some((Offset::now(), action.actor, None));
            },
            OrderAction::Unassign => {
                self.assigned = None;
            },
            OrderAction::MarkAsDelivered => {
                self.delivered = Some((action.actor, None, Offset::now()));
            },
            OrderAction::ConfirmDelivery => {
                self.delivery_confirmed_at = Some(Offset::now())
            },
            OrderAction::Delete => {
                panic!("should be handled by the database")
            },
        }

        Ok(prev_status)
    }
}

/// OrderAction for specific order
#[derive(Clone, Debug)]
pub struct SpecificAction {
    pub actor: UserId,
    pub order_id: OrderId,
    pub action: OrderAction,
}

impl SpecificAction {
    const BTN_DATA_PREFIX: &'static str = "oa";
    fn human_name(&self) -> &'static str {
        self.action.human_name()
    }

    /// Serializes it in a way that can be parsed by `try_parse`
    fn kbd_button_data(&self) -> String {
        format!("{} {} {}",
                Self::BTN_DATA_PREFIX,
                self.action.id(),
                self.order_id.0)
    }

    /// If `data` can be parsed as SpecificAtion it returns it, otherwise None
    ///
    /// `actor` is passed in separately because passing it as data is
    /// probably not safe, and it can be found in the callback
    pub fn try_parse(data: &str, actor: UserId) -> Option<SpecificAction> {
        let mut args = data.split(' ');

        let magic = args.next()?;
        if magic != Self::BTN_DATA_PREFIX { return None }

        let action = args.next()?;
        let order_id = args.next()?;
        let order_id = OrderId(order_id.parse().ok()?);

        // Too many arguments
        if args.next().is_some() { return None }

        let action = OrderAction::maybe_from_id(action)?;
        Some(SpecificAction { actor, action, order_id })
    }
}

fn actions_keyboard_markup(actions: &[SpecificAction]) -> InlineKeyboardMarkup {
    let btns: Vec<InlineKeyboardButton> = actions
        .iter()
        .map(|a| InlineKeyboardButton::callback(a.human_name(),
                                                a.kbd_button_data()) )
        .collect();
    let rows: Vec<Vec<InlineKeyboardButton>> = btns.chunks(2).map(|c| c.to_vec()).collect();
    InlineKeyboardMarkup::new(rows)
}

fn format_username(user: &User) -> String {
    let first_name = &user.first_name;
    let name = if let Some(last_name) = &user.last_name {
        format!("{first_name} {last_name}")
    } else {
        first_name.to_string()
    };

    if let Some(username) = &user.username {
        format!("@{username} {name}")
    } else {
        name
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
        let other = UserId(3);
        let oid = OrderId(1);

        let (msg, from) = mk_msg();
        let order = Order {
            id: Some(oid),
            created_at: chrono::offset::Utc::now(),
            price_in_drams: 0,
            canceled_at: None,
            delivered: None,
            published_at: None,
            urgency: Urgency::Whatever,
            desc_msg: msg,
            from: from.clone(),
            assigned: None,
            delivery_confirmed_at: None,
        };

        let act = |order: &mut Order, action: OrderAction, actor: UserId, expected_status: Status| {
            order.perform_action(&SpecificAction {
                action: action,
                actor: actor,
                order_id: oid,
            }).unwrap();
            assert_eq!(expected_status, order.status());
        };

        // happy path
        {
            let mut order = order.clone();

            act(&mut order, OrderAction::Publish,         publisher, Status::Published);
            act(&mut order, OrderAction::AssignToMe,      assignee,  Status::Assigned);
            act(&mut order, OrderAction::MarkAsDelivered, assignee,  Status::MarkedAsDelivered);
            act(&mut order, OrderAction::ConfirmDelivery, publisher, Status::DeliveryConfirmed);
        }

        // happy path, but confirmed without marking as delivered
        {
            let mut order = order.clone();

            act(&mut order, OrderAction::Publish,         publisher, Status::Published);
            act(&mut order, OrderAction::AssignToMe,      assignee,  Status::Assigned);
            act(&mut order, OrderAction::ConfirmDelivery, publisher, Status::DeliveryConfirmed);
        }
    }
}
