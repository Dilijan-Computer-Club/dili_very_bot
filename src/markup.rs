use teloxide::types::{User, UserId};
use chrono::Duration;
use askama_escape::{escape, Html, Escaped};
use crate::{DateTime, Offset};
use std::borrow::Cow;

pub fn format_amd(amd: u64) -> String {
    if amd == 0 {
        return "No payments required".to_string()
    }

    format!("{amd} AMD")
}

pub fn format_username(user: &User) -> String {
    let first_name = &user.first_name;
    let name = if let Some(last_name) = &user.last_name {
        format!("{first_name} {last_name}")
    } else {
        first_name.to_string()
    };

    if let Some(username) = &user.username {
        format!("@{username} {name}")
    } else {
        name
    }
}

pub fn link<U: AsRef<str>, N: AsRef<str>>(url: U, name: N) -> String {
    let name = escape_html(name.as_ref());
    let url = url.as_ref();
    format!("<a href=\"{url}\">{name}</a>")
}

pub fn user_url(uid: UserId) -> String {
    format!("tg://user?id={uid}")
}

pub fn user_link(user: &User) -> String {
    let url = user_url(user.id);
    let name = format_username(user);
    link(url, name)
}

pub fn escape_html(s: &str) -> Escaped<'_, Html> {
    escape(s, Html)
}

pub fn time_ago(t: DateTime) -> String {
    let now = Offset::now();
    let dur = t.signed_duration_since(now);
    if dur.is_zero() {
        return "right now".to_string();
    }
    let is_future = dur > Duration::zero();
    let dur = if is_future { dur } else { -dur };

    let amount = human_positive_duration(dur);
    let future_or_past = if is_future { "from now" } else { "ago" };
    format!("{amount} {future_or_past}")
}

/// Give me number and a word, I give you plural word
fn pluralize<'a>(n: u64, what: &'a str) -> Cow<'a, str> {
    let plur = || -> Cow<'a, str> { format!("{what}s").into() };
    let sing = || -> Cow<'a, str> { what.into() };
    match n {
        0 => plur(),
        1 => sing(),
        _ => plur(),
    }
}

pub fn human_positive_duration(dur: Duration) -> String {
    if dur.num_weeks() > 0 {
        let w = dur.num_weeks() as u64;
        return format!("{} {}", w, pluralize(w, "week"));
    }

    if dur.num_days() > 0 {
        let d = dur.num_days() as u64;
        return format!("{} {}", d, pluralize(d, "day"));
    }

    if dur.num_hours() > 0 {
        let h = dur.num_hours() as u64;
        return format!("{} {}", h, pluralize(h, "hour"));
    }

    if dur.num_minutes() > 0 {
        let m = dur.num_minutes() as u64;
        return format!("{} {}", m, pluralize(m, "minute"));
    }

    if dur.num_seconds() > 30 {
        return "about a minute".to_string()
    }

    if dur > Duration::zero() {
        return "few seconds".to_string()
    }

    "just now".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_humanize_positive_duration() {
        assert_eq!("4 weeks".to_string(),
                   human_positive_duration(Duration::weeks(4)));
        assert_eq!("5 days".to_string(),
                   human_positive_duration(Duration::days(5)));
        assert_eq!("1 minute".to_string(),
                   human_positive_duration(Duration::minutes(1)
                                           + Duration::seconds(5)));
    }
}
