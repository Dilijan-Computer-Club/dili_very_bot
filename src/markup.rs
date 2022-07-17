use teloxide::types::{User, UserId};
use std::fmt::Display;

pub fn link<U: AsRef<str>, N: Display>(url: U, name: N) -> String {
    let url = url.as_ref();
    format!("<a href=\"{url}\">{name}</a>")
}

pub fn user_url(uid: UserId) -> String {
    format!("tg://user?id={uid}")
}

pub fn user_link(user: &User) -> String {
    let url = user_url(user.id);
    let name = crate::utils::format_username(user);
    link(url, name)
}
