#[macro_use]
extern crate log;

use std::io::{Cursor, Seek, SeekFrom};

use image::{ImageFormat, ImageReader};
use log::LevelFilter;
use teloxide::{
    dispatching::dialogue::{self, InMemStorage},
    macros::BotCommands,
    net::Download,
    payloads::CreateNewStickerSet,
    prelude::*,
    requests::MultipartRequest,
    types::{InputFile, InputSticker, StickerFormat, StickerType},
    utils::command::BotCommands as _,
};

type DialogueFr = Dialogue<State, InMemStorage<State>>;
type ErrorValue = Box<dyn std::error::Error + Send + Sync + 'static>;
type HandlerResult = Result<(), ErrorValue>;

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
                .branch(case![Command::Create].endpoint(create_start))
                .branch(case![Command::Delete].endpoint(delete_start)),
        )
        .branch(case![Command::Cancel].endpoint(cancel));

    let state_map = dptree::entry()
        .branch(case![State::CreateReceivePackBasename].endpoint(create_receive_pack_name))
        .branch(case![State::CreateReceiveEmoji { pack_basename }].endpoint(create_receive_emoji))
        .branch(case![State::CreateReceivePicture { pack_basename, emoji }].endpoint(create_receive_picture))
        .branch(case![State::DeleteReceivePackName].endpoint(delete_receive_pack_name));

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

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
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
enum State {
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

async fn create_start(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    let mess = "Send me the identifier for your pack - something like \"my-cool-emojis\".";
    bot.send_message(msg.chat.id, mess).await?;
    diag.update(State::CreateReceivePackBasename).await?;
    Ok(())
}

async fn delete_start(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Send me the identifier of the emoji-pack to nuke. Completions currently unavailable due to skill issues, sorry.",
    )
    .await?;
    diag.update(State::DeleteReceivePackName).await?;
    Ok(())
}

async fn cancel(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    diag.exit().await?;
    let mess = "Cancelled whatever was going on.";
    bot.send_message(msg.chat.id, mess).await?;
    Ok(())
}

async fn delete_receive_pack_name(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    let Some(pack_name) = msg.text().map(ToOwned::to_owned) else {
        bot.send_message(msg.chat.id, "Please try again.").await?;
        return Ok(());
    };

    if let Ok(_) = bot.delete_sticker_set(pack_name).await {
        let mess = "All good! The nuke has reached its destination.";
        bot.send_message(msg.chat.id, mess).await?;
    } else {
        let mess = "Hmm, couldn't find that emoji pack. Cancelling operation.";
        bot.send_message(msg.chat.id, mess).await?;
    }

    diag.exit().await?;
    Ok(())
}

async fn create_receive_pack_name(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    let pack_basename = match msg.text().map(ToOwned::to_owned) {
        Some(basename) if (6..=24).contains(&basename.len()) && basename.is_ascii() => basename,
        _ => {
            let mess = "Not good. Maybe too long or too short? Try again.";
            bot.send_message(msg.chat.id, mess).await?;
            return Ok(());
        }
    };

    let pack_name = format!("{}_by_{}", pack_basename, bot_username());
    if let Ok(_) = bot.get_sticker_set(pack_name).await {
        let mess = ":warning: This pack already exists. /cancel unless you wish to overwrite its contents.";
        bot.send_message(msg.chat.id, mess).await?;
    }

    let mess = "Send me the emoji you want to fill the pack with.";
    bot.send_message(msg.chat.id, mess).await?;
    diag.update(State::CreateReceiveEmoji { pack_basename }).await?;

    Ok(())
}

async fn create_receive_emoji(bot: Bot, diag: DialogueFr, pack_basename: String, msg: Message) -> HandlerResult {
    match msg.text().map(ToOwned::to_owned) {
        Some(emoji) if (1..=4).contains(&emoji.len()) => {
            let mess = "Now send me the picture you want to slice. Attach it as a PNG file.";
            bot.send_message(msg.chat.id, mess).await?;

            let state = State::CreateReceivePicture { pack_basename, emoji };
            diag.update(state).await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Not good. Try again.").await?;
        }
    }
    Ok(())
}

async fn create_receive_picture(
    bot: Bot,
    diag: DialogueFr,
    (pack_basename, emoji): (String, String),
    msg: Message,
) -> HandlerResult {
    let pack_name = format!("{}_by_{}", pack_basename, bot_username());

    match msg.document() {
        None => {
            let mess = "Attach the picture as a PNG file please.";
            bot.send_message(msg.chat.id, mess).await?;
        }
        Some(ref pic) => {
            bot.send_message(msg.chat.id, "Processing...").await?;

            let file = bot.get_file(pic.file.id.clone()).await?;
            let mut data = Cursor::new(Vec::with_capacity(pic.file.size as usize));
            bot.download_file(&file.path, &mut data).await?;

            data.seek(SeekFrom::Start(0))?;
            let mut reader = ImageReader::new(data);
            reader.set_format(ImageFormat::Png);

            let image = match reader.decode() {
                Ok(image) => image,
                Err(err) => {
                    let mess = format!("Failed to parse your image. Try again.\n\nError code: `{:?}`", err);
                    bot.send_message(msg.chat.id, mess).await?;
                    return Ok(());
                }
            };

            let result = match emojify_tg_sticker::transform(&image) {
                Ok(result) => result,
                Err(err) => {
                    let mess = format!("Failed to process your image. Try again.\n\nError code: `{:?}`", err);
                    bot.send_message(msg.chat.id, mess).await?;
                    return Ok(());
                }
            };

            let user_id = msg.from.map(|x| x.id).ok_or("failed to get sender id")?;
            let stickers: Vec<InputSticker> = result
                .emojis
                .into_iter()
                .filter_map(|image| {
                    let mut cursor = Cursor::new(Vec::<u8>::new());
                    if let Err(_) = image.write_to(&mut cursor, ImageFormat::Png) {
                        return None;
                    }
                    if let Err(_) = cursor.seek(SeekFrom::Start(0)) {
                        return None;
                    }
                    Some(InputSticker {
                        sticker: InputFile::read(cursor),
                        format: StickerFormat::Static,
                        emoji_list: vec![emoji.to_string()],
                        mask_position: None,
                        keywords: vec![],
                    })
                })
                .collect();

            if let Ok(stickerset) = bot.get_sticker_set(&pack_name).await {
                bot.send_message(msg.chat.id, "Uploading... (overwriting existing emojis in the pack)")
                    .await?;

                for idx in 0..stickerset.stickers.len() {
                    bot.replace_sticker_in_set(
                        user_id,
                        &pack_name,
                        stickerset.stickers[idx].file.id.to_string(),
                        stickers[idx].clone(),
                    )
                    .await?;
                }

                if stickerset.stickers.len() < stickers.len() {
                    bot.send_message(msg.chat.id, "Uploading... (appending trailing emojis to the pack)")
                        .await?;
                }
                for idx in stickerset.stickers.len()..stickers.len() {
                    bot.add_sticker_to_set(user_id, &pack_name, stickers[idx].clone())
                        .await?;
                }
            } else {
                let req = CreateNewStickerSet {
                    user_id,
                    stickers,
                    title: format!("{} | TODO: edit", pack_basename),
                    name: pack_name.clone(),
                    sticker_type: Some(StickerType::CustomEmoji),
                    needs_repainting: None,
                };

                bot.send_message(msg.chat.id, "Uploading...").await?;
                MultipartRequest::new(bot.clone(), req).send().await?;
            }

            let mess = format!("All good! Try your emoji pack at t.me/addstickers/{pack_name}");
            bot.send_message(msg.chat.id, mess).await?;
            diag.exit().await?;
        }
    }

    Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "???").await?;
    Ok(())
}

fn bot_username() -> String {
    std::env::var("BOT_USERNAME").unwrap_or_else(|_| {
        error!("BOT_USERNAME unspecified; falling back to a garbage value");
        String::from("helloWorldBot")
    })
}
