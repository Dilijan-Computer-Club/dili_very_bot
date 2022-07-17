use teloxide::types::{User, UserId};

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

use askama_escape::{escape, Html, Escaped};
pub fn escape_html(s: &str) -> Escaped<'_, Html> {
    escape(s, Html)
}
