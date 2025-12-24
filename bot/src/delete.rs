use crate::prelude::*;

pub async fn start(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Send me the identifier of the emoji-pack to nuke. Completions currently unavailable due to skill issues, sorry.",
    )
    .await?;
    diag.update(State::DeleteReceivePackName).await?;
    Ok(())
}

pub async fn receive_pack_name(bot: Bot, diag: DialogueFr, msg: Message) -> HandlerResult {
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
