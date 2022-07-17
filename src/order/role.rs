
use crate::order::ActionKind;

#[derive(Clone, Copy, Debug)]
pub enum Role {
    Owner,
    Assignee,
    UnrelatedUser,
}

impl Role {
    pub const fn allowed_actions(self) -> &'static [ActionKind] {
        match self {
            Role::Owner =>
                &[ActionKind::Publish,
                  ActionKind::Cancel,
                  ActionKind::ConfirmDelivery,
                  ActionKind::Delete],
            Role::Assignee =>
                &[ActionKind::Unassign,
                ActionKind::MarkAsDelivered],
            Role::UnrelatedUser => &[ActionKind::AssignToMe],
        }
    }
}

