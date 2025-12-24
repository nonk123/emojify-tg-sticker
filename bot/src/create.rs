use std::io::{Cursor, Seek, SeekFrom};

use image::{ImageFormat, ImageReader};
use teloxide::{
    net::Download,
    payloads::CreateNewStickerSet,
    requests::MultipartRequest,
    types::{Document, InputFile, InputSticker, StickerFormat, StickerType},
};

use crate::prelude::*;

pub async fn start(bot: Bot, diag: DialogueFr, msg: Message) -> BotResult {
    let mess = "Send me the identifier for your pack - something like \"my-cool-emojis\".";
    bot.send_message(msg.chat.id, mess).await?;
    diag.update(State::CreateReceivePackId).await?;
    Ok(())
}

pub async fn receive_pack_id(bot: Bot, diag: DialogueFr, msg: Message) -> BotResult {
    let pack_id = match msg.text().map(ToOwned::to_owned) {
        Some(basename) if (6..=24).contains(&basename.len()) && basename.is_ascii() => basename,
        _ => {
            let mess = "Not good. Maybe too long or too short? Try again.";
            bot.send_message(msg.chat.id, mess).await?;
            return Ok(());
        }
    };

    if let Ok(_) = bot.get_sticker_set(pack_full_id(&pack_id)).await {
        let mess = "⚠️ This pack already exists. Send /cancel unless you wish to overwrite its contents.";
        bot.send_message(msg.chat.id, mess).await?;
    }

    let mess = "Send me the emoji you want to fill the pack with.";
    bot.send_message(msg.chat.id, mess).await?;
    diag.update(State::CreateReceiveEmoji { pack_id }).await?;

    Ok(())
}

pub async fn receive_emoji(bot: Bot, diag: DialogueFr, pack_id: String, msg: Message) -> BotResult {
    match msg.text().map(ToOwned::to_owned) {
        Some(emoji) if (1..=4).contains(&emoji.len()) => {
            let mess = "Now send me the picture you want to slice. Attach it as a PNG file.";
            bot.send_message(msg.chat.id, mess).await?;

            let state = State::CreateReceivePicture { pack_id, emoji };
            diag.update(state).await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Not good. Try again.").await?;
        }
    }
    Ok(())
}

pub async fn receive_picture(bot: Bot, diag: DialogueFr, (id, emoji): (String, String), msg: Message) -> BotResult {
    match msg.document() {
        None => {
            let mess = "Attach the picture as a PNG file please.";
            bot.send_message(msg.chat.id, mess).await?;
        }
        Some(pic) => {
            bot.send_message(msg.chat.id, "Processing...").await?;

            let mess = if let Err(err) = upload_stickerset(pic.clone(), bot.clone(), &id, &emoji, msg.clone()).await {
                format!(
                    "{}\n{}",
                    markdown::escape("Something went wrong; cancelling operation. Full error message:"),
                    markdown::blockquote(&err.to_string())
                )
            } else {
                format!(
                    "All good! Try your emoji pack at t.me/addstickers/{}",
                    pack_full_id(&id)
                )
            };

            bot.send_message(msg.chat.id, mess)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            diag.exit().await?;
        }
    }
    Ok(())
}

async fn upload_stickerset(pic: Document, bot: Bot, id: &str, emoji: &str, msg: Message) -> BotResult<()> {
    let file = bot.get_file(pic.file.id.clone()).await?;
    let mut data = Cursor::new(Vec::with_capacity(pic.file.size as usize));
    bot.download_file(&file.path, &mut data).await?;

    data.seek(SeekFrom::Start(0))?;
    let mut reader = ImageReader::new(data);
    reader.set_format(ImageFormat::Png);

    let user_id = msg.from.map(|x| x.id).ok_or("Failed to get sender id")?;
    let image = reader.decode()?;

    let stickers: Vec<InputSticker> = emojify_tg_sticker::transform(&image)?
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

    if let Ok(stickerset) = bot.get_sticker_set(pack_full_id(id)).await {
        bot.send_message(msg.chat.id, "Uploading... (overwriting existing emojis in the pack)")
            .await?;

        for idx in 0..stickerset.stickers.len() {
            bot.replace_sticker_in_set(
                user_id,
                pack_full_id(id),
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
            bot.add_sticker_to_set(user_id, &pack_full_id(id), stickers[idx].clone())
                .await?;
        }
    } else {
        let req = CreateNewStickerSet {
            user_id,
            stickers,
            title: format!("{} | TODO: edit", id),
            name: pack_full_id(id),
            sticker_type: Some(StickerType::CustomEmoji),
            needs_repainting: None,
        };

        bot.send_message(msg.chat.id, "Uploading...").await?;
        MultipartRequest::new(bot.clone(), req).send().await?;
    }
    Ok(())
}

fn pack_full_id(id_from_user: &str) -> String {
    format!("{}_by_{}", id_from_user, crate::bot_username())
}
