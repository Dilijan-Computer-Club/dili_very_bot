
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderAction {
    /// Show this order in the list of all orders
    Publish,

    /// Don't need to deliver it any more
    Cancel,

    /// Agree to deliver it
    AssignToMe,

    /// Refuse to deliver it
    Unassign,

    /// As assignee we say that we've delivered it
    MarkAsDelivered,

    /// As the owner we confirm that it's delivered
    ConfirmDelivery,

    /// Delete it completely
    Delete,
}

impl OrderAction {
    pub const fn human_name(&self) -> &'static str {
        match self {
            OrderAction::Publish         => "Publish this order",
            OrderAction::Cancel          => "Cancel this order",
            OrderAction::AssignToMe      => "Take this order",
            OrderAction::Unassign        => "Unassign this order",
            OrderAction::MarkAsDelivered => "Mark as delivered",
            OrderAction::ConfirmDelivery => "Confirm that I've received the items",
            OrderAction::Delete          => "Delete this order",
        }
    }

    pub const fn id(&self) -> &'static str {
        match self {
            OrderAction::Publish         => "publish",
            OrderAction::Cancel          => "cancel",
            OrderAction::AssignToMe      => "assign_to_me",
            OrderAction::Unassign        => "unassign",
            OrderAction::MarkAsDelivered => "mark_as_delivered",
            OrderAction::ConfirmDelivery => "confirm_delivery",
            OrderAction::Delete          => "delete",
        }
    }

    /// Converts str to OrderAction, returns None if it doesn't
    /// match any of the variant ids
    pub fn maybe_from_id<S: AsRef<str>>(s: S) -> Option<OrderAction> {
        match s.as_ref() {
            "publish"           => Some(OrderAction::Publish),
            "cancel"            => Some(OrderAction::Cancel),
            "assign_to_me"      => Some(OrderAction::AssignToMe),
            "unassign"          => Some(OrderAction::Unassign),
            "mark_as_delivered" => Some(OrderAction::MarkAsDelivered),
            "confirm_delivery"  => Some(OrderAction::ConfirmDelivery),
            "delete"            => Some(OrderAction::Delete),
            _other              => None
        }
    }
}
