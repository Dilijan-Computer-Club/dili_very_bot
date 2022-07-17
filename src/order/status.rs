
use std::fmt;

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

