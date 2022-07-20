use teloxide::{
    prelude::*,
    types::User,
    dispatching::UpdateHandler,
};

use serde::{Serialize, Deserialize};

use std::num::ParseIntError;
use std::num::IntErrorKind;

use crate::error::Error;
use crate::MyDialogue;
use crate::db::Db;
use crate::order::Order;
use crate::ui;
use crate::ui::commands::Command;
use crate::utils;
use crate::Offset;

type HandlerResult = Result<(), Error>;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum State {
    #[default]
    Start,
    ReceivedName {
        name: String },
    ReceivedPrice {
        name: String, price: u64, },
    ReceivedMarkup {
        name: String, price: u64, markup: u64, },
    ReceivedDescription {
        name: String, price: u64, markup: u64, description: String },
}

pub fn schema() -> UpdateHandler<Error> {
    // Add /cancel command to all messages
    let message_handler = Update::filter_message()
        .branch(dptree::case![State::Start]
                .endpoint(receive_name))
        .branch(dptree::case![State::ReceivedName  { name }]
                .endpoint(receive_price))
        .branch(dptree::case![State::ReceivedPrice { name, price }]
                .endpoint(receive_markup))
        .branch(dptree::case![State::ReceivedMarkup  { name, price, markup }]
                .endpoint(receive_description));

    dptree::entry()
        .branch(message_handler)
}



/// Start new order in either private or public chat
///
/// If the `cid` is a public chat then we send a private message
/// suggesting to create the order in private
pub async fn start(
    bot: AutoSend<Bot>,
    db: Db,
    dialogue: MyDialogue,
    cid: ChatId,
    uid: UserId,
) -> HandlerResult {
    if cid.is_user() {
        // Make sure user's in a public chat before asking them anything
        let _ = pub_chat_or_bail(bot.clone(), dialogue.clone(), db, cid, uid)
            .await?;

        dialogue.update(
            ui::State::NewOrder(ui::new_order::State::default())).await?;
        ui::new_order::send_initial_message(
            bot.clone(), cid).await?;
    } else {
        // It was clicied in a public chat, so:
        //
        // 1. Send a private message suggesting to start a new order
        // 2. Send back a public message suggesting to check
        //    private messages. We later delete this message.
        bot.send_message(utils::uid_to_cid(uid),
        format!("Create new order here with {} command",
                Command::NewOrder)).await?;

        let msg = bot.send_message(cid, "I've sent you a private message!")
            .await?;

        let msg_id = msg.id;
        tokio::spawn(async move {
            log::debug!("send_menu_link deleting the link");
            tokio::time::sleep(ui::TEMP_MSG_TIMEOUT).await;
            let _ = bot.delete_message(cid, msg_id).await;
        });
    }
    Ok(())
}

/// Send the first message of the dialogue for creating new order.
///
/// Must be sent only in a private chat
async fn send_initial_message(
    bot: AutoSend<Bot>,
    cid: ChatId)
-> HandlerResult {
    if !cid.is_user() {
        let msg = format!("Cannot send initial new_order \
message in a public chat {cid}");
        log::warn!("{}", msg);
        return Err(msg.into())
    }
    bot.send_message(cid, "What do you want?").await?;
    Ok(())
}

async fn receive_name(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: MyDialogue,
) -> HandlerResult {
    log::info!("-> receive_name");

    if msg.text().is_none() {
        bot.send_message(dialogue.chat_id(), "You haven't written your \
order's name. Please try again. Just write a message containing \
the name of your order").await?;
        return Ok(())
    }
    let text = msg.text().unwrap();

    ask_for_price(bot, dialogue.clone()).await?;
    change_state(
        dialogue, State::ReceivedName { name: text.to_string() }).await?;
    log::info!("received name: {text}");

    Ok(())
}

async fn ask_for_price(
    bot: AutoSend<Bot>,
    dialogue: MyDialogue,
) -> HandlerResult {
    bot.send_message(dialogue.chat_id(),
                     "How much is it in Armenian Drams? \
A rough estimate is enough. Say 0 if it's already paid for").await?;
    Ok(())
}

async fn receive_price(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: MyDialogue,
    name: String,
) -> HandlerResult {
    log::info!("-> receive_price {name}");

    if msg.text().is_none() {
        bot.send_message(dialogue.chat_id(),
        "Please send me the price of this order, I've reecived nothing")
            .await?;
        return Ok(())
    }
    let text = msg.text().unwrap();

    let price = parse_price(text);
    if let Err(e) = price {
        bot.send_message(dialogue.chat_id(),
            format!("I don't understand the price - {e}, please try again"))
            .await?;
        return Ok(())
    }
    let price = price.unwrap();

    ask_for_markup(bot, dialogue.clone()).await?;
    change_state(
        dialogue, State::ReceivedPrice { name, price }).await?;

    Ok(())
}

async fn ask_for_markup(
    bot: AutoSend<Bot>,
    dialogue: MyDialogue,
) -> HandlerResult {
    bot.send_message(dialogue.chat_id(),
                     "How much (Drams) will you offer for the delivery?
It is completely optional, say 0 for no markup.").await?;
    Ok(())
}

async fn receive_markup(
    bot: AutoSend<Bot>,
    msg: Message,
    dialogue: MyDialogue,
    name_price: (String, u64),
) -> HandlerResult {
    log::info!("-> receive_markup {name_price:?}");
    if msg.text().is_none() {
        bot.send_message(dialogue.chat_id(),
        "Please send me how much above the item price are you \
willing to pay for the delivery, I've reecived nothing").await?;
        return Ok(())
    }
    let text = msg.text().unwrap();

    let markup = parse_price(text);
    if let Err(e) = markup {
        bot.send_message(dialogue.chat_id(),
            format!("I don't understand the price - {e}, please try again"))
            .await?;
        return Ok(())
    }
    let markup = markup.unwrap();

    ask_for_description(bot, dialogue.clone()).await?;
    let (name, price) = name_price;
    change_state(
        dialogue, State::ReceivedMarkup { name, price, markup }).await?;

    Ok(())
}

async fn ask_for_description(
    bot: AutoSend<Bot>,
    dialogue: MyDialogue,
) -> HandlerResult {
    bot.send_message(dialogue.chat_id(),
                     "Write some details of the item you want delivered, \
where to get it from and other important details.").await?;
    Ok(())
}

async fn receive_description(
    bot: AutoSend<Bot>,
    dialogue: MyDialogue,
    db: Db,
    msg: Message,
    name_price_markup: (String, u64, u64),
) -> HandlerResult {
    log::info!("-> receive_description {name_price_markup:?}");
    if msg.text().is_none() {
        bot.send_message(dialogue.chat_id(),
        "Please write a description.
We don't allow photos or videos right now. Sorry!").await?;
        return Ok(())
    }
    let description_text = msg.text().unwrap().to_string();

    let (name, price_in_drams, markup_in_drams) = name_price_markup;
    let user = msg.from();
    if user.is_none() {
        log::warn!("receive_price No user in msg {msg:?}");
        return Err(format!("No user is msg {msg:?}").into());
    }
    let user = user.unwrap();
    let order_data = OrderData {
        name, price_in_drams, markup_in_drams, description_text,
    };
    finish_creating_order(
        bot, db, dialogue, user, order_data).await?;

    Ok(())
}

struct OrderData {
    name: String,
    price_in_drams: u64,
    markup_in_drams: u64,
    description_text: String,
}

/// Gets a public chat or leaves the dialogue
async fn pub_chat_or_bail(
    bot: AutoSend<Bot>,
    dialogue: MyDialogue,
    mut db: Db,
    cid: ChatId,
    uid: UserId,
) -> Result<(ChatId, String), Error> {
    let pub_chats = db.user_public_chats(uid).await?;

    if pub_chats.len() == 1 {
        return Ok(pub_chats[0].clone());
    }

    if pub_chats.is_empty() {
        let log_msg = "User {uid} is not in any pub chat";
        log::warn!("{log_msg}");
        bot.send_message(cid,
            format!("I don't see you in any public chats.
Try sending {} to the public chat I'm in.", Command::Hello)).await?;
        exit_dialogue(dialogue).await?;
        ui::main_menu::send_menu_link(bot, cid).await?;
        return Err(log_msg.into());
    }

    bot.send_message(cid,
        format!("You're in multiple public chats {} and \
we don't support it yet", pub_chats.len())).await?;
    let msg = "TODO: Support multiple pub chats uid = {uid}";
    log::warn!("{msg}");
    exit_dialogue(dialogue).await?;
    ui::main_menu::send_menu_link(bot, cid).await?;
    Err(msg.into())
}

async fn finish_creating_order(
    bot: AutoSend<Bot>,
    mut db: Db,
    dialogue: MyDialogue,
    user: &User,
    order_data: OrderData,
) -> HandlerResult {
    let OrderData { name, price_in_drams,
        markup_in_drams, description_text } = order_data;
    log::info!("-> finish_creating_order {name} \
{price_in_drams} {markup_in_drams}");

    let mut order = Order {
        id: None,
        name,
        description_text,
        price_in_drams,
        markup_in_drams,
        created_at: Offset::now(),
        published_at: None,
        customer: user.clone(),
        assigned: None,
        delivered: None,
        delivery_confirmed_at: None,
        canceled_at: None,
    };
    let uid = user.id;

    let cid = dialogue.chat_id();
    let pub_chat = pub_chat_or_bail(
        bot.clone(), dialogue.clone(), db.clone(), cid, uid).await?;
    let pcid = pub_chat.0;
    let oid = db.add_order(pcid, &mut order).await?;
    let mut order = order;
    order.id = Some(oid);

    ui::order::send_message(db, &order, bot.clone(),
    Some(uid), dialogue.chat_id(),
    Some("New Order is created! You need to publish it \
before other people can see it")).await?;
    exit_dialogue(dialogue).await?;
    ui::main_menu::send_menu_link(bot, cid).await?;

    Ok(())
}

async fn change_state(dialogue: MyDialogue, state: State) -> HandlerResult {
    dialogue.update(ui::State::NewOrder(state)).await?;
    Ok(())
}

async fn exit_dialogue(dialogue: MyDialogue) -> HandlerResult {
    dialogue.update(ui::State::Start).await?;
    Ok(())
}

/// Transform int parse error into something more price-speccific
fn parse_price(text: &str) -> Result<u64, Error> {
    let price: Result<u64, ParseIntError> = text.parse();
    match price {
        Ok(price) => Ok(price),
        Err(e) => {
            let e = match e.kind() {
                IntErrorKind::InvalidDigit => "invalid symbol, only numbers are allowed",
                IntErrorKind::Empty        => "you haven't written anything",
                IntErrorKind::PosOverflow  => "ain't no one got that much money",
                IntErrorKind::NegOverflow  => "way too little moneys",
                other => {
                    log::warn!("Weird parse int error: {other:?}");
                    "some weird error occured"
                },
            };

            Err(e.into())
        }
    }
}

