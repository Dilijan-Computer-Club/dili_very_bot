pub fn dumb_intersection<T: Clone + PartialEq>(aa: &[T], bb: &[T]) -> Vec<T> {
    let mut res = Vec::with_capacity(aa.len().max(bb.len()));
    for a in aa.iter() {
        for b in bb.iter() {
            if a == b { res.push(a.clone()) }
        }
    }
    res
}

use teloxide::types::{ChatId, UserId};
pub fn uid_to_cid(uid: UserId) -> ChatId {
    let cid = ChatId(uid.0 as i64);
    if ! cid.is_user() {
        panic!("Chat_id is not user after converting from chat_id")
    }
    cid
}
