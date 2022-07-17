
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
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

impl ActionKind {
    pub const fn human_name(&self) -> &'static str {
        match self {
            ActionKind::Publish         => "Publish this order",
            ActionKind::Cancel          => "Cancel this order",
            ActionKind::AssignToMe      => "Take this order",
            ActionKind::Unassign        => "Unassign this order",
            ActionKind::MarkAsDelivered => "Mark as delivered",
            ActionKind::ConfirmDelivery => "Confirm that I've received the items",
            ActionKind::Delete          => "Delete this order",
        }
    }

    pub const fn id(&self) -> &'static str {
        match self {
            ActionKind::Publish         => "publish",
            ActionKind::Cancel          => "cancel",
            ActionKind::AssignToMe      => "assign_to_me",
            ActionKind::Unassign        => "unassign",
            ActionKind::MarkAsDelivered => "mark_as_delivered",
            ActionKind::ConfirmDelivery => "confirm_delivery",
            ActionKind::Delete          => "delete",
        }
    }

    /// Converts str to ActionKind, returns None if it doesn't
    /// match any of the variant ids
    pub fn maybe_from_id<S: AsRef<str>>(s: S) -> Option<ActionKind> {
        match s.as_ref() {
            "publish"           => Some(ActionKind::Publish),
            "cancel"            => Some(ActionKind::Cancel),
            "assign_to_me"      => Some(ActionKind::AssignToMe),
            "unassign"          => Some(ActionKind::Unassign),
            "mark_as_delivered" => Some(ActionKind::MarkAsDelivered),
            "confirm_delivery"  => Some(ActionKind::ConfirmDelivery),
            "delete"            => Some(ActionKind::Delete),
            _other              => None
        }
    }
}
