
use crate::order::OrderId;
use std::fmt;

#[derive(Clone, Copy, Debug)]
pub enum ActionError {
    /// Public chat was specified but it doesn't exist in db
    PubChatNotFound,

    /// Could not find the specified order
    OrderNotFound(OrderId),

    /// User is not allowed to perform that action
    NotPermitted,

    /// Some other technical error
    Other
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // write!(f, "({}, {})", self.x, self.y)
        match self {
            ActionError::PubChatNotFound => {
                write!(f, "Could not find this public chat. Is the bot still there?")
            },
            ActionError::OrderNotFound(_) => {
                write!(f, "Could not find this order. 
Either you clicked on a stale message or it's a bug (oh noes!)")
            },
            ActionError::NotPermitted => {
                write!(f, "You are not permitted to perform this action")
            },
            ActionError::Other => { write!(f, "Some error occured") }
        }
    }
}
