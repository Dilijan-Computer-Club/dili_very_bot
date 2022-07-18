use teloxide::{
    prelude::*,
    dispatching::UpdateHandler,
};

use crate::db::Db;
use crate::order::Order;
use crate::ui;
use std::num::ParseIntError;
use std::num::IntErrorKind;
use crate::error::Error;
use crate::MyDialogue;
use crate::data_gathering;
use serde::{Serialize, Deserialize};

type HandlerResult = Result<(), Error>;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum State {
    #[default]
    Start, // receive name
    ReceivedName { name: String },
    ReceivedPrice { name: String, price: u64, }
}

pub fn schema() -> UpdateHandler<Error> {
    // Add /cancel command to all messages
    let message_handler = Update::filter_message()
        .branch(dptree::case![State::Start]
                .endpoint(receive_name))
        .branch(dptree::case![State::ReceivedName { name }]
                .endpoint(receive_price));

    let callback_query_handler = Update::filter_callback_query()
        .branch(dptree::case![State::ReceivedPrice { name, price }]
                .endpoint(receive_urgency));

    dptree::entry()
        .branch(message_handler)
        .branch(callback_query_handler)
}

pub async fn send_initial_message(
    bot: AutoSend<Bot>,
    chat_id: ChatId)
-> HandlerResult {
    bot.send_message(chat_id, "What do you want?").await?;
    Ok(())
}

async fn receive_name(
    bot: AutoSend<Bot>,
    msg: Message,
    _db: Db,
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

    log::info!("Description: {text}");
    bot.send_message(dialogue.chat_id(),
                     "How much is it in Armenian Drams? \
A rough estimate is enough. Say 0 if it's already paid for").await?;
    change_state(dialogue, State::ReceivedName { name: text.to_string() }).await?;

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

    let price: Result<u64, ParseIntError> = text.parse();
    if let Err(e) = price {
        let e = match e.kind() {
            IntErrorKind::InvalidDigit =>
                "invalid symbol, only numbers are allowed",
            IntErrorKind::Empty       => "you haven't written anything",
            IntErrorKind::PosOverflow => "ain't no one got that much money",
            IntErrorKind::NegOverflow => "way too little moneys",
            other => {
                log::warn!("Weird parse int error: {other:?}");
                "some weird error occured"
            },

        };
        bot.send_message(dialogue.chat_id(),
            format!("I don't understand the price - {e}, please try again"))
            .await?;
        return Ok(())
    }
    let price = price.unwrap();

    let buttons = ui::urgency::keyboard_markup();
    bot.send_message(dialogue.chat_id(), "How soon you need it?")
        .reply_markup(buttons)
        .await?;

    change_state(dialogue, State::ReceivedPrice { name, price }).await?;

    Ok(())
}

async fn receive_urgency(
    mut bot: AutoSend<Bot>,
    q: CallbackQuery,
    mut db: Db,
    dialogue: MyDialogue,
    name_price: (String, u64),
) -> HandlerResult {
    let (name, price) = name_price;
    log::info!("-> receive_urgency {name} {price}");
    let data = q.data;
    if data.is_none() {
        log::warn!("receive_urgency got callback without data");
        bot.send_message(dialogue.chat_id(),
            "Something weird happened, please try again").await?;
        return Ok(());
    }
    let data = data.unwrap();
    log::info!("data = \"{data}\"");
    let urgency = ui::urgency::from_id(&data);
    if urgency.is_none() {
        log::warn!("receive_urgency got invalid urgency {urgency:?}");
        bot.send_message(dialogue.chat_id(),
            "Something weird happened, please try again").await?;
        return Ok(());
    }
    let urgency = urgency.unwrap();
    let uid = q.from.id;

    // Just for testing
    let mut order = Order {
        id: None,
        desc_msg: crate::tg_msg::TgMsg {
            chat_id: ChatId(0),
            message_id: 0,
            text: name,
        },
        urgency,
        price_in_drams: price,
        created_at: crate::Offset::now(),
        published_at: None,
        from: q.from,
        assigned: None,
        delivered: None,
        delivery_confirmed_at: None,
        canceled_at: None,
    };

    let pub_chats = db.user_public_chats(uid).await?;
    // TODO handle 0 pub chats too

    if pub_chats.is_empty() {
        log::warn!("User {uid} is not in any pub chat");
        bot.send_message(dialogue.chat_id(),
            "TODO: I don't see you in any public chats").await?;
        exit_dialogue(dialogue).await?;
        return Ok(())
    }

    if pub_chats.len() == 1 {
        let pcid = pub_chats[0].0;
        let oid = db.add_order(pcid, &mut order).await?;
        let mut order = order;
        order.id = Some(oid);

        ui::order::send_message(&order, &mut bot, Some(uid), dialogue.chat_id(),
            Some("New Order is created! You need to publish it \
before other people can see it")).await?;
        exit_dialogue(dialogue).await?;
        return Ok(())
    }

    bot.send_message(dialogue.chat_id(),
        format!("You're in multiple public chats {} and we don't support it yet", pub_chats.len())).await?;
    log::warn!("TODO: Support multiple pub chats uid = {uid}");

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

/// Tries to find pub chat id
async fn get_pcid(
    bot: &mut AutoSend<Bot>,
    dialogue: &MyDialogue,
    db: &mut Db,
    msg: &Message,
) -> Result<ChatId, Error> {
    let pcid = data_gathering::pub_chat_id_from_msg(db, msg.clone()).await;
    match pcid {
        Err(e) => {
            log::warn!(" -> recv_desc: Could not get pcid from msg {:?}", &msg);
            bot.send_message(dialogue.chat_id(), format!("{e}")).await?;
            Err(format!("{e}").into())
        },
        Ok(pcid) => Ok(pcid),
    }
}

