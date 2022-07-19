use teloxide::{
    prelude::*,
    types::User,
};
use crate::order::{self, Order};
use crate::Db;
use crate::error::Error;
use crate::ui::{self, MyDialogue};
use crate::markup;
use crate::utils;

/// Handles order button clicks
///
/// Returns true if order is changed and we need to delete the old message
/// to avoid confusion
pub async fn handle_order_action(
    mut bot: AutoSend<Bot>,
    uid: UserId,
    pcid: ChatId,
    action: order::Action,
    mut db: Db,
    dialogue: MyDialogue,
) -> Result<bool, Error> {
    let action_type = action.kind;
    let res = db.perform_action(uid, pcid, action).await;
    log::info!("db.perform_action => {res:?}");
    if let Err(e) = res {
        log::warn!("handle_order_action perform_action({uid}, {pcid}) => {e:?}");
        // handle error here
        bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
        return Ok(false)
    }
    let (prev_status, order) = res.unwrap();

    if order.is_none() {
        bot.send_message(dialogue.chat_id(), "Deleted the order").await?;
        return Ok(false)
    }
    let order = order.unwrap();
    let new_status = order.status();

    match new_status {
        order::Status::Unpublished => {
            bot.send_message(
                dialogue.chat_id(),
                "The order is unpublished. Now it's not shown to anybody.")
                .await?;
        },
        order::Status::Published => {
            order_published_notifications(
                &mut bot, dialogue.chat_id(), pcid, &order).await?;
        },
        order::Status::Assigned => {
            order_assigned_notifications(
                &mut bot, db, uid,
                pcid, &order).await?;
        },
        order::Status::MarkedAsDelivered => {
            // Send message to the chat in which it was marked as delivered
            bot.send_message(dialogue.chat_id(),
            "Order is marked as delivered. It will be closed after \
the publisher confirms they've received it.").await?;

            // Send message to the owner asking to confirm delivery
            let assignee_link = get_assignee_link(db, &order).await?;
            let msg = format!("{assignee_link} marked order as delivered. \
Please confirm it.");

            let priv_chat_id: ChatId = ChatId(order.from.id.0 as i64);
            ui::order::send_message(
                &order, &mut bot, Some(uid), priv_chat_id, Some(msg)).await?;


        },
        order::Status::DeliveryConfirmed => {
            delivery_confirmed_notifications(&mut bot, &order).await?;
        },
    }

    // bot.send_message(dialogue.chat_id(),
    //     format!("Changed status to {new_status}")).await?;
    log::info!("Order status update: {prev_status} + {action_type:?} -> \
{new_status}    {order:?}");

    Ok(prev_status != new_status)
}

pub async fn order_published_notifications(
    bot: &mut AutoSend<Bot>,
    chat_id: ChatId,
    pcid: ChatId,
    order: &Order,
) -> Result<(), Error> {
    if pcid != chat_id {
        // It's a different chat, so reply here as well as send
        // a notification to the main public chat

        // If the chat id value is the same as the owner of the order
        // it means we're in private chat with this user and should
        // show actions according to their permissions
        let uid = if chat_id.0 == order.from.id.0 as i64 {
            Some(order.from.id)
        } else {
            None
        };

        ui::order::send_message(
            order, bot, uid, chat_id,
            Some("New order is published")).await?;
    }

    // Send notification to public chat
    ui::order::send_message(order, bot, None, pcid,
                            Some("New order is published")).await?;

    Ok(())
}
pub async fn order_assigned_notifications(
    bot: &mut AutoSend<Bot>,
    db: Db,
    uid: UserId,
    pcid: ChatId,
    order: &Order,
) -> Result<(), Error> {
    let assignee_link = get_assignee_link(db, order).await?;

    // Send a private message to the order owner
    {
        let msg = format!("Order is assigned to {assignee_link}");

        let priv_chat_id: ChatId = uid.into();
        ui::order::send_message(
            order, bot, Some(uid), priv_chat_id, Some(msg)).await?;

        // Send a public message sayng the order is taken
        let msg = format!("Order is taken by {assignee_link}");
        ui::order::send_message(
            order, bot, None, pcid, Some(msg)).await?;
    }

    // Send message to the owner
    {
        let owner_uid = order.from.id;
        let owner_cid = utils::uid_to_cid(owner_uid);
        if owner_cid.is_none() {
            log::warn!("Order ({:?}) owner ({owner_cid:?}) \
could not be converted to priv chat????", order.id);
        }
        let owner_cid = owner_cid.unwrap();

        let msg = format!("Congrats! {assignee_link} has agreed to deliver \
your order! Feel free to send them a message.");
        ui::order::send_message(
            order, bot, Some(owner_uid), owner_cid, Some(msg)).await?;
    }

    Ok(())
}

pub async fn delivery_confirmed_notifications(
    bot: &mut AutoSend<Bot>,
    order: &Order,
) -> Result<(), Error> {
    // Send message to the assignee
    let assignee_id = order.assigned.as_ref().unwrap().1;
    let assignee_cid = utils::uid_to_cid(assignee_id);
    if let Some(assignee_cid) = assignee_cid {
        ui::order::send_message(
            order, bot, Some(assignee_id), assignee_cid,
            Some("Order delivery is confirmed! Thank you!")).await?;
    } else {
        log::warn!("Assignee {assignee_id} is not user???");
    }
    // Send message to the owner
    let owner_id = order.from.id;
    let owner_cid = utils::uid_to_cid(owner_id);
    if let Some(owner_cid) = owner_cid {
        ui::order::send_message(
            order, bot, Some(owner_id), owner_cid,
            Some("Order delivery is confirmed!")).await?;
    }
    Ok(())
}

async fn get_assignee_link(
    mut db: Db,
    order: &Order,
) -> Result<String, Error> {
    let assignee_uid = order.assigned.as_ref().unwrap().1;
    let assignee: Option<User> = db.get_user(assignee_uid).await?;
    if assignee.is_none() {
        // No assignee, this must be an error
        let msg = format!("Couldn't find assignee {assignee_uid}");
        log::warn!("{}", msg);
        return Err(msg.into());
    }
    let assignee = assignee.unwrap();
    let assignee_link = markup::user_link(&assignee);
    Ok(assignee_link)
}

