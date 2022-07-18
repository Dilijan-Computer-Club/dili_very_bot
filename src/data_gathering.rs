use teloxide::prelude::*;
use teloxide::types::{MessageKind, MessageNewChatMembers, ChatKind,
                      MessageLeftChatMember};
use crate::error::Error;
use crate::Db;
use crate::db::PubChatFromMsgError;

// TODO remove
pub async fn pub_chat_id_from_msg(
    db: &mut Db,
    msg: Message,
) -> Result<ChatId, PubChatFromMsgError> {
    log::info!("-> pub_chat_id_from_msg");

    // if msg.is public then that's it
    // otherwise if user is present find them in chats, if there is one
    //   then that's it
    // return appropriate error otherwise

    if let ChatKind::Public(_) = msg.chat.kind {
        return Ok(msg.chat.id)
    }

    let user = msg.from();
    if user.is_none() { return Err(PubChatFromMsgError::Other) }
    let user = user.unwrap();

    let pc = db.user_public_chats(user.id).await;
    if let Err(e) = pc {
        log::warn!("pub_chat_id_from_msg: {e:?}");
        return Err(PubChatFromMsgError::Other)
    }
    let pc: Vec<(ChatId, String)> = pc.unwrap();

    if pc.is_empty() {
        return Err(PubChatFromMsgError::NotInPubChats)
    }
    if pc.len() > 1 {
        return Err(PubChatFromMsgError::MultipleChats)
    }

    log::info!("-> pub_chat_id_from_msg => {pc:?}");
    Ok(pc[0].0)
}

pub async fn pub_chat_id_from_cq(
    db: &mut Db,
    q: CallbackQuery,
    ) -> Result<ChatId, PubChatFromMsgError> {
    log::info!("-> pub_chat_id_from_cq");


    // if msg.is public then that's it
    // otherwise if user is present find them in chats, if there is one
    //   then that's it
    // return appropriate error otherwise
    let msg = &q.message;
    if msg.is_none() {
        return Err(PubChatFromMsgError::Other)
    }
    let msg = msg.as_ref().unwrap();

    if let ChatKind::Public(_) = msg.chat.kind {
        return Ok(msg.chat.id)
    }

    let user = &q.from;
    let pc = db.user_public_chats(user.id).await;
    if let Err(e) = pc {
        log::warn!("pub_chat_id_from_msg: {e:?}");
        return Err(PubChatFromMsgError::Other)
    }
    let pc: Vec<(ChatId, String)> = pc.unwrap();

    if pc.is_empty() {
        return Err(PubChatFromMsgError::NotInPubChats)
    }
    if pc.len() > 1 {
        return Err(PubChatFromMsgError::MultipleChats)
    }

    log::info!("-> pub_chat_id_from_cq => {pc:?}");
    Ok(pc[0].0)
}

pub async fn collect_data_from_cq(
    db: &mut Db,
    cq: CallbackQuery,
) -> Result<(), Error> {
    log::debug!("-> collect_data_from_cq {cq:?}");

    db.update_user(cq.from.clone()).await?;

    if let Some(msg) = &cq.message {
        collect_data_from_msg(db, msg.clone()).await?;
        if ! msg.chat.id.is_user() {
            db.add_members(msg.chat.id, vec![cq.from.id]).await?;
        }
    }
    Ok(())
}

pub async fn collect_data_from_msg(
    db: &mut Db,
    msg: Message,
) -> Result<(), Error> {
    log::debug!("-> collect_data_from_msg, {msg:?}");

    if let Some(user) = msg.from()  {
        db.update_user(user.clone()).await?;
    }

    let cid = msg.chat.id;
    db.update_chat(msg.clone().chat).await?;
    match &msg.kind {
        MessageKind::NewChatMembers(payload) => {
            handle_new_chat_members(db, cid, payload).await?;
        },
        MessageKind::LeftChatMember(payload) => {
            handle_left_chat_members(db, cid, payload).await?;
        },
        MessageKind::Common(m) => {
            if let Some(chat) = &m.sender_chat {
                db.update_chat(chat.clone()).await?;
            }
        },
        _ => {},
    }
    Ok(())
}

async fn handle_new_chat_members(
    db: &mut Db,
    pcid: ChatId,
    m: &MessageNewChatMembers
) -> Result<(), Error> {
    for u in m.new_chat_members.iter() {
        db.update_user(u.clone()).await?;
    }

    let uids = m.new_chat_members.iter().map(|m| m.id).collect();
    db.add_members(pcid, uids).await
}

async fn handle_left_chat_members(
    db: &mut Db,
    cid: ChatId,
    m: &MessageLeftChatMember
) -> Result<(), Error> {
    let user = &m.left_chat_member;
    let uid = user.id;

    db.update_user(user.clone()).await?;
    db.remove_chat_membership(cid, uid).await?;

    Ok(())
}
