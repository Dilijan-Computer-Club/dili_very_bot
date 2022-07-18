use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Urgency {
    Today,
    ThisWeek,
    ThisMonth,
    Whenever
}

impl Urgency {
    pub const ALL: &'static [Urgency] =
        &[ Urgency::Today,
           Urgency::ThisWeek,
           Urgency::ThisMonth,
           Urgency::Whenever ];

    pub const fn id(self) -> &'static str {
        match self {
            Urgency::Today     => "today",
            Urgency::ThisWeek  => "this_week",
            Urgency::ThisMonth => "this_month",
            Urgency::Whenever  => "whenever",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Urgency::Today     => "Today",
            Urgency::ThisWeek  => "Some time this week",
            Urgency::ThisMonth => "Some time this month",
            Urgency::Whenever  => "Some day",
        }
    }

    pub fn from_id(id: &str) -> Option<Urgency> {
        Urgency::ALL.iter().cloned().find(|u| u.id() == id)
    }
}

impl fmt::Display for Urgency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_roundtrips() {
        for u in Urgency::ALL.iter().cloned() {
            assert_eq!(u, Urgency::from_id(u.id()).unwrap());
        }
    }

    #[test]
    fn test_some_cases() {
        assert_eq!(Some(Urgency::ThisWeek), Urgency::from_id("this_week"));
        assert_eq!(None,                    Urgency::from_id("this_week "));
        assert_eq!(None,                    Urgency::from_id(" this_week "));
        assert_eq!(None,                    Urgency::from_id(" this_week "));
        assert_eq!(None,                    Urgency::from_id("eueue"));
    }
}
