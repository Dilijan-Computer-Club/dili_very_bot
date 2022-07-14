
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
}

