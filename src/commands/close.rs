use serenity::{
    async_trait,
    client::Context,
    futures::StreamExt,
    http::AttachmentType,
    model::{
        id::{ChannelId, UserId},
        interactions::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction,
        },
    },
};

use super::{Button, Command};

pub struct Close;

#[async_trait]
impl Command for Close {
    fn name(&self) -> String {
        String::from("close")
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::CONFIG
            .guild
            .create_application_command(&ctx.http, |command| {
                command
                    .name(self.name())
                    .description("Closes a screenshare")
            })
            .await?;
        Ok(())
    }
    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let channel = command.channel_id;
        let close = close_ticket(&ctx, command.user.id, channel).await?;
        if !close {
            command
                .create_interaction_response(&ctx.http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content("This is not a screenshare ticket!")
                    })
                })
                .await?;
        }
        Ok(())
    }
    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

#[async_trait]
impl Button for Close {
    async fn click(
        &self,
        ctx: &Context,
        command: &MessageComponentInteraction,
    ) -> crate::Result<()> {
        close_ticket(&ctx, command.user.id, command.channel_id).await?;
        Ok(())
    }
}

pub async fn close_ticket(
    ctx: &Context,
    closer: UserId,
    channel: ChannelId,
) -> crate::Result<bool> {
    let screenshare = crate::consts::DATABASE.fetch_screenshares_for(channel.0);
    if screenshare.is_none() {
        return Ok(false);
    }
    let mut messages = Vec::new();
    let raw_messages: Vec<_> = channel.messages_iter(&ctx.http).collect().await;

    for message in raw_messages {
        if let Ok(msg) = message {
            messages.push(format!(
                "{}: {}",
                msg.author.tag(),
                msg.content_safe(&ctx.cache).await
            ));
        }
    }

    messages.reverse();
    let history = messages.join("\n").into_bytes().into();
    let screenshare = screenshare.unwrap();
    crate::CONFIG
        .ss_logs
        .send_message(&ctx.http, |msg| {
            msg.files([AttachmentType::Bytes {
                data: history,
                filename: String::from("messages.txt"),
            }])
            .embed(|embed| {
                embed.title("Screenshare closed").description(format!(
                    "- Creator: <@{}>
- In Question: <@{}>
- Closer: <@{}>",
                    screenshare.creator, screenshare.in_question, closer
                ))
            })
        })
        .await?;
    crate::consts::DATABASE.remove_entry("Screenshares", channel.0)?;
    channel.delete(&ctx.http).await?;
    Ok(true)
}
