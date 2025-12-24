use crate::prelude::*;

pub async fn start(bot: Bot, diag: DialogueFr, msg: Message) -> BotResult {
    let mess = "Send me the identifier of the emoji-pack to nuke. Completions currently unavailable due to skill issues, sorry.";
    bot.reply_to(&msg, mess).await?;
    diag.update(State::DeleteReceivePackName).await?;
    Ok(())
}

pub async fn receive_pack_name(bot: Bot, diag: DialogueFr, msg: Message) -> BotResult {
    let Some(pack_name) = msg.text().map(ToOwned::to_owned) else {
        bot.reply_to(&msg, "Please try again.").await?;
        return Ok(());
    };

    if let Ok(_) = bot.delete_sticker_set(pack_name).await {
        bot.reply_to(&msg, "All good! The nuke has reached its destination.").await?;
    } else {
        bot.reply_to(&msg, "Hmm, couldn't find that emoji pack. Cancelling operation.").await?;
    }

    diag.exit().await?;
    Ok(())
}
