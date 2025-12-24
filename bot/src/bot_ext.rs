use teloxide::types::ParseMode;

use crate::prelude::*;

pub trait BotExt {
    async fn reply_to(&self, msg: &Message, contents: impl Into<String>) -> BotResult;
    async fn md_reply_to(&self, msg: &Message, contents: impl Into<String>) -> BotResult;
}

impl BotExt for Bot {
    async fn reply_to(&self, msg: &Message, contents: impl Into<String>) -> BotResult {
        self.send_message(msg.chat.id, contents.into()).await?;
        Ok(())
    }

    async fn md_reply_to(&self, msg: &Message, contents: impl Into<String>) -> BotResult {
        self.send_message(msg.chat.id, contents.into())
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        Ok(())
    }
}
