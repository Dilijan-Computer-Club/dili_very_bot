use std::error::Error;
use teloxide::{
    prelude::*,
    payloads::SendMessageSetters,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup,
        InlineQueryResultArticle, InputMessageContent,
        InputMessageContentText,
    },
    dispatching::{
        dialogue::{self, InMemStorage, Storage},
        UpdateHandler
    },
    utils::command::BotCommands,
};

mod error;
mod new_order;
mod order;
mod order_action;
mod tg_msg;
mod db;
mod urgency;


use db::Db;
pub use order::Order;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(BotCommands, Clone)]
#[command(rename = "snake_case",
          description = "These commands are supported:")]
enum Command {
    #[command(description = "Start here")]
    Start,
    #[command(description = "Show main menu")]
    Menu,
    #[command(description = "Show help")]
    Help,
    #[command(description = "List all orders")]
    ListAllOrders,
    #[command(description = "Show my orders")]
    ListMyOrders,
    #[command(description = "Make New Order")]
    MakeNewOrder,
}

#[derive(Clone, Default, Debug)]
enum State {
    #[default]
    Start,
    MainMenu,
    NewOrder(new_order::State)
}

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type MyStorage = InMemStorage<State>;

fn init_bot() -> Result<Bot, Box<dyn Error>> {
    use std::io::Read;
    let mut file = std::fs::File::open("key")?;
    let mut key = String::new();
    file.read_to_string(&mut key)?;
    Ok(Bot::new(key))
}

pub fn schema() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            dptree::case![State::Start]
                .branch(dptree::case![Command::Menu].endpoint(main_menu))
                .branch(dptree::case![Command::Start].endpoint(main_menu))
                .branch(dptree::case![Command::ListAllOrders].endpoint(list_all_orders))
        );

    let message_handler = Update::filter_message()
        .branch(command_handler);

    let callback_query_handler = Update::filter_callback_query().chain(
        dptree::case![State::MainMenu].endpoint(handle_main_menu),
    );

    dialogue::enter::<Update, MyStorage, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
        .branch(dptree::case![State::NewOrder(no)].branch(new_order::schema()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let db = Db::new();

    let bot = init_bot()?.auto_send();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![MyStorage::new(), db])
        .build() // .setup_ctrlc_handler()
        .dispatch()
        .await;

    Ok(())
}

enum MainMenuItem {
    ListAllOrders,
    ShowMyOrders,
    NewOrder,
}

impl MainMenuItem {
    pub const fn human_name(&self) -> &'static str {
        match self {
            MainMenuItem::ListAllOrders => "List all orders",
            MainMenuItem::ShowMyOrders  => "Show my orders",
            MainMenuItem::NewOrder      => "New Order",
        }
    }

    pub const fn id(&self) -> &'static str {
        match self {
            MainMenuItem::ListAllOrders => "list_all_orders",
            MainMenuItem::ShowMyOrders  => "show_my_orders",
            MainMenuItem::NewOrder      => "new_order",
        }
    }

    pub const fn all_items() -> [Self; 3] {
        [
            MainMenuItem::ListAllOrders,
            MainMenuItem::ShowMyOrders,
            MainMenuItem::NewOrder,
        ]
    }

    pub fn from_id(s: &str) -> Option<MainMenuItem> {
        match s {
          "list_all_orders" => Some(MainMenuItem::ListAllOrders),
          "show_my_orders"  => Some(MainMenuItem::ShowMyOrders),
          "new_order"       => Some(MainMenuItem::NewOrder),
          _ => None
        }
    }
}

/// Shows the main menu with buttons
async fn main_menu(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    log::info!("-> main_menu");
    let main_menu_items = MainMenuItem::all_items()
        .map(|item| [InlineKeyboardButton::callback(item.human_name(), item.id())]);

    bot.send_message(msg.chat.id, "Choose your destiny")
        .reply_markup(InlineKeyboardMarkup::new(main_menu_items)).
        await?;
    dialogue.update(State::MainMenu).await?;
    Ok(())
}

async fn list_all_orders(
    mut bot: AutoSend<Bot>,
    db: Db,
    uid: UserId,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> list_all_orders");
    let orders = db.list_all_orders()?;
    if orders.len() == 0 {
        bot.send_message(dialogue.chat_id(), "No orders")
            .await?;
    } else {
        bot.send_message(dialogue.chat_id(), "All current orders:").await?;
        for order in orders.iter() {
            order.send_message_for(&mut bot, uid, dialogue.chat_id()).await?;
        }
    }
    dialogue.update(State::Start).await?;
    Ok(())
}

async fn show_my_orders(
    mut bot: AutoSend<Bot>,
    db: Db,
    uid: UserId,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> show_my_orders");
    let orders = db.orders_submitted_by_user(uid)?;
    if orders.len() == 0 {
        bot.send_message(dialogue.chat_id(), "You have no current orders")
            .await?;
    } else {
        bot.send_message(dialogue.chat_id(), "Your orders:").await?;
        for order in orders.iter() {
            order.send_message_for(&mut bot, uid, dialogue.chat_id()).await?;
        }
    }
    dialogue.update(State::Start).await?;
    Ok(())
}

async fn handle_main_menu(
    bot: AutoSend<Bot>,
    q: CallbackQuery,
    db: Db,
    dialogue: MyDialogue
) -> HandlerResult {
    log::info!("-> handle_main_menu, query: {q:?}");
    let uid = q.from.id;
    if let Some(msg) = q.message {
        bot.delete_message(dialogue.chat_id(), msg.id).await?;
    }
    if let Some(item) = q.data {
        let menu_item = MainMenuItem::from_id(&item);
        match menu_item {
            Some(MainMenuItem::NewOrder) => {
                dialogue.update(
                    State::NewOrder(new_order::State::default())).await?;
                new_order::send_initial_message(bot, dialogue.chat_id()).await?;
            },
            Some(MainMenuItem::ShowMyOrders) => {
                show_my_orders(bot, db, q.from.id, dialogue).await?;
            },
            Some(MainMenuItem::ListAllOrders) => {
                list_all_orders(bot, db, uid, dialogue).await?;
            },
            None => {
                panic!("TODO wrong button \"{}\"", item.as_str())
            }
        }
    } else {
        // What do we do here? Nothing?
    }
    Ok(())
}
