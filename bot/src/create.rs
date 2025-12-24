use std::io::{Cursor, Seek, SeekFrom};

use image::{ImageFormat, ImageReader};
use teloxide::{
    net::Download,
    payloads::CreateNewStickerSet,
    requests::MultipartRequest,
    types::{InputFile, InputSticker, StickerFormat, StickerType},
};

use crate::prelude::*;

pub async fn start(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    let mess = "Send me the identifier for your pack - something like \"my-cool-emojis\".";
    bot.send_message(msg.chat.id, mess).await?;
    diag.update(State::CreateReceivePackBasename).await?;
    Ok(())
}

pub async fn receive_pack_name(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    let pack_basename = match msg.text().map(ToOwned::to_owned) {
        Some(basename) if (6..=24).contains(&basename.len()) && basename.is_ascii() => basename,
        _ => {
            let mess = "Not good. Maybe too long or too short? Try again.";
            bot.send_message(msg.chat.id, mess).await?;
            return Ok(());
        }
    };

    let pack_name = format!("{}_by_{}", pack_basename, crate::bot_username());
    if let Ok(_) = bot.get_sticker_set(pack_name).await {
        let mess = "⚠️ This pack already exists. /cancel unless you wish to overwrite its contents.";
        bot.send_message(msg.chat.id, mess).await?;
    }

    let mess = "Send me the emoji you want to fill the pack with.";
    bot.send_message(msg.chat.id, mess).await?;
    diag.update(State::CreateReceiveEmoji { pack_basename }).await?;

    Ok(())
}

pub async fn receive_emoji(bot: Bot, diag: DialogueFr, pack_basename: String, msg: Message) -> HandlerResult {
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

pub async fn receive_picture(
    bot: Bot,
    diag: DialogueFr,
    (pack_basename, emoji): (String, String),
    msg: Message,
) -> HandlerResult {
    let pack_name = format!("{}_by_{}", pack_basename, crate::bot_username());

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
