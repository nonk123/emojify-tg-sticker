#[macro_use]
extern crate log;

use std::io::{Cursor, Seek, SeekFrom};

use color_eyre::eyre::OptionExt;
use image::{ImageFormat, ImageReader};
use log::LevelFilter;
use teloxide::{
    dispatching::dialogue::{self, InMemStorage},
    macros::BotCommands,
    net::Download,
    payloads::CreateNewStickerSet,
    prelude::*,
    requests::MultipartRequest,
    types::{InputFile, InputSticker, StickerFormat, StickerType::CustomEmoji, True},
    utils::command::BotCommands as _,
};

type DialogueFr = Dialogue<State, InMemStorage<State>>;
type HandlerResult<T = ()> = color_eyre::Result<T>;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let _ = dotenvy::dotenv();
    let _ = color_eyre::install();
    env_logger::builder().filter_level(LevelFilter::Info).init();

    info!("Starting the bot");
    let bot = Bot::from_env();
    let storage = InMemStorage::<State>::new();

    use dptree::{case, endpoint};

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::Start]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Create].endpoint(create_start))
                .branch(case![Command::Delete].endpoint(delete_start)),
        )
        .branch(case![Command::Cancel].endpoint(cancel));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::CreateReceivePackBasename].endpoint(create_receive_pack_name))
        .branch(case![State::CreateReceiveEmoji { pack_basename }].endpoint(create_receive_emoji))
        .branch(case![State::CreateReceivePicture { pack_basename, emoji }].endpoint(create_receive_picture))
        .branch(case![State::DeleteReceivePackName].endpoint(delete_receive_pack_name))
        .branch(endpoint(invalid_state));

    let dialogue_handler = dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(message_handler);

    Dispatcher::builder(bot, dialogue_handler)
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
    /// Nuke one of the created emoji pack.
    Delete,
    /// Cancel the ongoing procedure (if any).
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
    bot.send_message(msg.chat.id, "Try /help to figure out what to do with me.")
        .await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    let mess = format!("{}", Command::descriptions().to_string());
    bot.send_message(msg.chat.id, mess).await?;
    Ok(())
}

async fn create_start(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Send me the identifier for your pack - something like \"my-cool-emojis\".",
    )
    .await?;
    diag.update(State::CreateReceivePackBasename).await?;
    Ok(())
}

async fn delete_start(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Send me the identifier of the emoji-pack to nuke.")
        .await?;
    diag.update(State::DeleteReceivePackName).await?;
    Ok(())
}

async fn cancel(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Cancelled whatever was going on.")
        .await?;
    diag.exit().await?;
    Ok(())
}

async fn delete_receive_pack_name(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    let Some(pack_name) = msg.text().map(ToOwned::to_owned) else {
        bot.send_message(msg.chat.id, "Please try again.").await?;
        return Ok(());
    };
    if let Ok(True) = bot.delete_sticker_set(pack_name).await {
        bot.send_message(msg.chat.id, "All good! Nuke has been received.")
            .await?;
        diag.exit().await?;
    } else {
        bot.send_message(
            msg.chat.id,
            "Hmm, couldn't find that emoji pack. Try again? Or /cancel.",
        )
        .await?;
        diag.update(State::DeleteReceivePackName).await?;
    }
    Ok(())
}

async fn create_receive_pack_name(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    let pack_basename = match msg.text().map(ToOwned::to_owned) {
        Some(pack_basename) => pack_basename,
        None => {
            bot.send_message(msg.chat.id, "Not good. Try again.").await?;
            return Ok(());
        }
    };

    let pack_name = format!("{}_by_{}", pack_basename, bot_username());
    if let Ok(_) = bot.get_sticker_set(pack_name).await {
        let mess = "This pack already exists. Either nuke it or try another name.";
        bot.send_message(msg.chat.id, mess).await?;
        return Ok(());
    }

    bot.send_message(msg.chat.id, "Send me the emoji you want to fill the pack with.")
        .await?;
    diag.update(State::CreateReceiveEmoji { pack_basename }).await?;

    Ok(())
}

async fn create_receive_emoji(bot: Bot, diag: DialogueFr, pack_basename: String, msg: Message) -> HandlerResult {
    match msg.text().map(ToOwned::to_owned) {
        Some(emoji) => {
            let mess = "Now send me the picture you want to slice. Attach it as a PNG file.";
            bot.send_message(msg.chat.id, mess).await?;
            diag.update(State::CreateReceivePicture { pack_basename, emoji })
                .await?;
        }
        None => {
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
            bot.send_message(msg.chat.id, "Attach the picture as a PNG file please.")
                .await?;
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

            if let Ok(_) = bot.get_sticker_set(&pack_name).await {
                let _ = bot.delete_sticker_set(&pack_name).await;
            }

            let user_id = msg.from.map(|x| x.id).ok_or_eyre("failed to get sender id")?;
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

            bot.send_message(msg.chat.id, "Uploading...").await?;
            MultipartRequest::new(
                bot.clone(),
                CreateNewStickerSet {
                    user_id,
                    stickers,
                    title: format!("{} | TODO: edit", pack_basename),
                    name: pack_name.clone(),
                    sticker_type: Some(CustomEmoji),
                    needs_repainting: None,
                },
            )
            .send()
            .await?;

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
        warn!("BOT_USERNAME unspecified; falling back to a garbage value");
        String::from("helloWorldBot")
    })
}
