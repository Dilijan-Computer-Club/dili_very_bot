use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, User}
};
use crate::tg_msg::TgMsg;
use crate::error::Error;
use crate::order_action::OrderAction;
use crate::urgency::Urgency;

// use chrono::{DateTime, offset::Utc};
use chrono;

type DateTime = chrono::DateTime<chrono::offset::Utc>;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct OrderId(u64);

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

#[derive(Clone, Copy, Debug)]
pub enum Status {
    Unpublished,
    Published,
    Assigned,
    MarkedAsDelivered,
    DeliveryConfirmed,
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

    /// User who agreed to deliver it
    pub assigned_to: Option<User>,

    /// When and by whom it was delivered, if it was
    pub delivered: Option<(User, DateTime)>,

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
            None => return Err(format!("No 'from' in message").into()),
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
            assigned_to: None,
            delivery_confirmed_at: None,
        })
    }

    pub fn description_text(&self) -> String {
        self.desc_msg.text.clone()
    }

    fn role(&self, uid: UserId) -> Role {
        if self.from.id == uid {
            return Role::Owner;
        }

        if let Some(assignee) = &self.assigned_to {
            if assignee.id == uid {
                return Role::Assignee;
            }
        }

        Role::UnrelatedUser
    }

    pub const fn status(&self) -> Status {
        if self.delivery_confirmed_at.is_some() {
            return Status::DeliveryConfirmed }
        if self.delivered.is_some() { return Status::MarkedAsDelivered }
        if self.assigned_to.is_some() { return Status::Assigned }
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
        uid: UserId
    ) -> Vec<OrderAction> {
        let role = self.role(uid);
        let available_actions = self.available_actions();
        let allowed_actions = role.allowed_actions();

        dumb_intersection(allowed_actions, available_actions)
    }

    pub async fn send_message_for(
        &self,
        bot: &mut AutoSend<Bot>,
        uid: UserId,
        chat_id: ChatId
    ) -> Result<(), Error> {
        let bot = bot.parse_mode(teloxide::types::ParseMode::Html);
        let description = &self.desc_msg.text;
        let username = format_username(&self.from);
        let user_id = self.from.id;
        let actions = self.user_actions(uid);
        let buttons = actions_keyboard_markup(&actions);

        let text = format!("\
<a href=\"tg://user?id={user_id}\">{username}</a>

{description}");
        bot.send_message(chat_id, text)
            .reply_markup(buttons).await?;
        Ok(())
    }
}

fn actions_keyboard_markup(actions: &[OrderAction]) -> InlineKeyboardMarkup {
    // TODO group by 2
    let btns: Vec<InlineKeyboardButton> = actions
        .iter()
        .map(|a| InlineKeyboardButton::callback(a.human_name(), a.id()) )
        .collect();
    let rows: Vec<Vec<InlineKeyboardButton>> = btns.chunks(2).map(|c| c.to_vec()).collect();
    InlineKeyboardMarkup::new(rows)
}

fn format_username(user: &User) -> String {
    let first_name = &user.first_name;
    let name = if let Some(last_name) = &user.last_name {
        format!("{first_name} {last_name}")
    } else {
        format!("{first_name}")
    };

    if let Some(username) = &user.username {
        format!("@{username} {name}")
    } else {
        format!("{name}")
    }
}

