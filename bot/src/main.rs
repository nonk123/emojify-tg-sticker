#[macro_use]
extern crate log;

use log::LevelFilter;
use prelude::*;
use teloxide::{
    dispatching::dialogue::{self, InMemStorage},
    macros::BotCommands,
    utils::command::BotCommands as _,
};

mod create;
mod delete;

pub mod prelude {
    pub type DialogueFr = Dialogue<crate::State, super::InMemStorage<crate::State>>;
    pub type ErrorValue = Box<dyn std::error::Error + Send + Sync + 'static>;
    pub type HandlerResult = Result<(), ErrorValue>;

    pub use crate::{Command, State};
    pub use teloxide::prelude::*;
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// Display the introduction.
    Start,
    /// Display the list of available commands.
    Help,
    /// Create or overwrite an emoji pack.
    Create,
    /// Nuke one of the created emoji packs.
    Delete,
    /// Cancel the ongoing operation (if any).
    Cancel,
}

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    CreateReceivePackBasename,
    CreateReceiveEmoji {
        pack_basename: String,
    },
    CreateReceivePicture {
        pack_basename: String,
        emoji: String,
    },
    DeleteReceivePackName,
}

#[tokio::main]
async fn main() -> HandlerResult {
    let _ = dotenvy::dotenv();
    env_logger::builder().filter_level(LevelFilter::Info).init();

    info!("starting the bot");

    let bot = Bot::from_env();
    bot.set_my_commands(Command::bot_commands()).await?;

    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::Start]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Create].endpoint(create::start))
                .branch(case![Command::Delete].endpoint(delete::start)),
        )
        .branch(case![Command::Cancel].endpoint(cancel));

    let state_map = dptree::entry()
        .branch(case![State::CreateReceivePackBasename].endpoint(create::receive_pack_name))
        .branch(case![State::CreateReceiveEmoji { pack_basename }].endpoint(create::receive_emoji))
        .branch(case![State::CreateReceivePicture { pack_basename, emoji }].endpoint(create::receive_picture))
        .branch(case![State::DeleteReceivePackName].endpoint(delete::receive_pack_name));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(state_map)
        .endpoint(invalid_state);

    let dialogue_handler = dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(message_handler);

    let storage = InMemStorage::<State>::new();
    Dispatcher::builder(bot.clone(), dialogue_handler)
        .dependencies(dptree::deps![storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn start(bot: Bot, msg: Message) -> HandlerResult {
    // TODO: say something useful instead.
    let mess = "See /help to figure out what to do with me.";
    bot.send_message(msg.chat.id, mess).await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    let mess = Command::descriptions().to_string();
    bot.send_message(msg.chat.id, mess).await?;
    Ok(())
}

async fn cancel(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    diag.exit().await?;
    let mess = "Cancelled whatever was going on.";
    bot.send_message(msg.chat.id, mess).await?;
    Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "???").await?;
    Ok(())
}

pub fn bot_username() -> String {
    std::env::var("BOT_USERNAME").unwrap_or_else(|_| {
        error!("BOT_USERNAME unspecified; falling back to a garbage value");
        String::from("helloWorldBot")
    })
}
