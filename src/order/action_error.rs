
use crate::order::OrderId;
use std::fmt;

#[derive(Clone, Copy, Debug)]
pub enum ActionError {
    /// We couldn't find a public chat for this user
    NoPublicChats,

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
            ActionError::NoPublicChats => {
                write!(f, "You are not in any public chat with this bot.
Try writting '/hello' into the public chat you're in to make sure \
the bot knows you're there")
            },
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
