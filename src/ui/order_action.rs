use teloxide::{
    prelude::*,
    types::{User, MessageId},
};
use crate::order::{self, Order, OrderId, ActionKind};
use crate::Db;
use crate::error::Error;
use crate::ui::{self, MyDialogue};
use crate::markup;
use crate::utils;
use crate::data_gathering;

/// If it's an order query then handle it and return `true`,
/// otherwise just return `false`
pub async fn try_handle_query(
    bot: AutoSend<Bot>,
    mut db: Db,
    dialogue: MyDialogue,
    q: CallbackQuery,
    data: &str,
) -> Result<bool, Error> {
    let action = order::Action::try_parse(data);
    if action.is_none() {
        return Ok(false)
    }
    let action = action.unwrap();

    log::info!("  got order action from callback query {action:?}");
    let pcid = data_gathering::pub_chat_id_from_cq(&mut db, q.clone()).await;
    if let Err(e) = pcid {
        log::warn!("-> handle_unknown_callback_query pcid: {e:?}");
        bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
        // Returning true because it's a right kind of query,
        // we just failed handling it
        return Ok(true)
    }
    let pcid = pcid.unwrap();

    let user = q.from;
    let changed = ui::order_action::handle_order_action(
        bot.clone(), user, pcid, action, db, dialogue).await?;

    if changed {
        if q.message.is_none() {
            log::warn!("Message is missing in callback query");
            return Ok(true)
        }

        // The order is changed so we better delete the old
        // messege showing the order
        let msg = q.message.unwrap();
        log::info!("deleting old order message {}", msg.id);
        bot.delete_message(msg.chat.id, msg.id).await?;
    }
    Ok(true)
}

/// Handles order button clicks
///
/// Returns true if order is changed and we need to delete the old message
/// to avoid confusion
async fn handle_order_action(
    bot: AutoSend<Bot>,
    user: User,
    pcid: ChatId,
    action: order::Action,
    mut db: Db,
    dialogue: MyDialogue,
) -> Result<bool, Error> {
    let uid = user.id;
    let action_type = action.kind;
    let oid: OrderId = action.order_id;

    // Special case for order deletion because we need to also try to delete
    // all messages where we posted this order
    // (or should we mark them as deleted somehow?
    if let ActionKind::Delete = action_type {
        let msgs: Vec<(ChatId, MessageId)> =
            db.order_msg_ids(oid).await?;

        for (cid, mid) in msgs.into_iter() {
            if let Err(e) = bot.delete_message(cid, mid.message_id).await {
                log::warn!("could not delete order message ({cid}, {mid:?}): {e:?}")
            }
        }
    }

    let res = db.perform_action(user, pcid, action).await;
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
                db.clone(), bot, dialogue.chat_id(), pcid, &order).await?;
        },
        order::Status::Assigned => {
            order_assigned_notifications(
                bot, db, uid,
                pcid, &order).await?;
        },
        order::Status::MarkedAsDelivered => {
            // Send message to the chat in which it was marked as delivered
            bot.send_message(dialogue.chat_id(),
            "Order is marked as delivered. It will be closed after \
the publisher confirms they've received it.").await?;

            // Send message to the owner asking to confirm delivery
            let assignee_link = get_assignee_link(db.clone(), &order).await?;
            let msg = format!("{assignee_link} marked order as delivered. \
Please confirm it.");

            let priv_chat_id: ChatId = ChatId(order.customer.id.0 as i64);
            ui::order::send_message(
                db, &order, bot, Some(uid), priv_chat_id, Some(msg)).await?;


        },
        order::Status::DeliveryConfirmed => {
            delivery_confirmed_notifications(db, bot, &order).await?;
        },
    }

    // bot.send_message(dialogue.chat_id(),
    //     format!("Changed status to {new_status}")).await?;
    log::info!("Order status update: {prev_status} + {action_type:?} -> \
{new_status}    {order:?}");

    Ok(prev_status != new_status)
}

pub async fn order_published_notifications(
    db: Db,
    bot: AutoSend<Bot>,
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
        let uid = if chat_id.0 == order.customer.id.0 as i64 {
            Some(order.customer.id)
        } else {
            None
        };

        ui::order::send_message(
            db.clone(), order, bot.clone(), uid, chat_id,
            Some("New order is published")).await?;
    }

    // Send notification to public chat
    ui::order::send_message(db, order, bot, None, pcid,
                            Some("New order is published")).await?;

    Ok(())
}
pub async fn order_assigned_notifications(
    bot: AutoSend<Bot>,
    db: Db,
    uid: UserId,
    pcid: ChatId,
    order: &Order,
) -> Result<(), Error> {
    let assignee_link = get_assignee_link(db.clone(), order).await?;

    // Send a private message to the order owner
    {
        let msg = format!("Order is assigned to {assignee_link}");

        let priv_chat_id: ChatId = uid.into();
        ui::order::send_message(
            db.clone(), order, bot.clone(), Some(uid),
            priv_chat_id, Some(msg)).await?;

        // Send a public message sayng the order is taken
        let msg = format!("Order is taken by {assignee_link}");
        ui::order::send_message(
            db.clone(), order, bot.clone(), None, pcid, Some(msg)).await?;
    }

    // Send message to the owner
    {
        let owner_uid = order.customer.id;
        let owner_cid = utils::uid_to_cid(owner_uid);

        let msg = format!("Congrats! {assignee_link} has agreed to deliver \
your order! Feel free to send them a message.");
        ui::order::send_message(
            db, order, bot, Some(owner_uid), owner_cid, Some(msg)).await?;
    }

    Ok(())
}

pub async fn delivery_confirmed_notifications(
    db: Db,
    bot: AutoSend<Bot>,
    order: &Order,
) -> Result<(), Error> {
    // Send message to the assignee
    let assignee_id = order.assigned.as_ref().unwrap().1;
    let assignee_cid = utils::uid_to_cid(assignee_id);
    ui::order::send_message(
        db.clone(), order, bot.clone(), Some(assignee_id), assignee_cid,
        Some("Order delivery is confirmed! Thank you!")).await?;
    // Send message to the owner
    let owner_id = order.customer.id;
    let owner_cid = utils::uid_to_cid(owner_id);
    ui::order::send_message(
        db, order, bot, Some(owner_id), owner_cid,
        Some("Order delivery is confirmed!")).await?;
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

