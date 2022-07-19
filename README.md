# Dilivery
Telegram bot for managing deliveries to Dilijan

## TODOs
 - Redis error when there is only 1 order in chat
 - `/cancel` to stop creating the order, or other things
 - `/help`
 - Edit own orders
 - Admin UI
 - Hide outdated orders
 - Support multiple group chats per user (ask chat on order creation)
 - Handle error when user doesn't have a public chat
 - Support multiple languages
 - Notifications to assignees
 - Add optional private instructions for order
 - Persistent storage
 - Subscribe to new orders

## Running
- Put a Telegram bot key into the `key` file
- `cargo run`

Set `RUST_LOG` to `info` or `debug` for more verbose logging.

## Usage
 - Create a group chat and invite the bot into it
 - Create orders by sending `/start` command in a private message to the bot
   and following the menu
- Find orders either in the group chat or in a private chat with the bot

## Bulid requirements
### Rust nightly
Easy to install via https://rustup.rs

### Debian / Ubuntu
```sh
apt install build-essential libssl-dev pkg-config
```
