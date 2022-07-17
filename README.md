# Dilivery
Telegram bot for managing deliveries to Dilijan

## TODOs
 - Status / debugging / stats information in tg
 - Delete order doesn't work?
 - Support multiple group chats per user (ask chat on order creation)
 - Handle error when user doesn't have a public chat
 - Support multiple languages
 - Notifications
 - Add optional private instructions for order
 - Persistent storage
 - Subscribe to new orders

## Running
- Put a Telegram bot key into the `key` file
- `cargo run`

## Usage
 - Create a group chat and invite the bot into it
 - Create orders by sending `/start` command in a private message to the bot
   and following the menu
- Find orders either in the group chat or in a private chat with the bot

## Bulid requirements
### Debian / Ubuntu
```sh
apt install build-essential libssl-dev pkg-config
```
