use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use crate::urgency::Urgency;

const BTN_PREFIX: &str = "urg";

pub fn id(u: &Urgency) -> String {
    format!("{BTN_PREFIX} {}", u.id())
}

pub fn from_id(u: &str) -> Option<Urgency> {
    let mut s = u.split(' ');
    let magic = s.next();
    if magic != Some(BTN_PREFIX) {
        return None;
    }
    let urg = s.next()?;
    let urg = Urgency::from_id(urg)?;
    if s.next().is_some() {
        // Unexpected elements
        return None;
    }
    Some(urg)
}

pub fn keyboard_markup() -> InlineKeyboardMarkup {
    let btns = Urgency::ALL.iter()
        .map(|u| [InlineKeyboardButton::callback(u.name(), id(u))] );
    InlineKeyboardMarkup::new(btns)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_id() {
        assert_eq!(Some(Urgency::ThisWeek), from_id("urg this_week"));
        assert_eq!(None,                    from_id("urg this_week "));
        assert_eq!(None,                    from_id(" urg this_week"));
        assert_eq!(None,                    from_id("xxx this_week"));
    }
}
