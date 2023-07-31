use futures::StreamExt;
use serenity::{
    async_trait,
    client::Context,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction,
        message_component::MessageComponentInteraction,
    },
    model::prelude::*,
};

use bridge_scrims::{interaction::*, print_embeds::FormatEmbed};

pub struct Close;

#[async_trait]
impl InteractionHandler for Close {
    fn name(&self) -> String {
        String::from("close")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec![crate::CONFIG.ss_support])
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

    fn initial_response(
        &self,
        _interaction_type: interaction::InteractionType,
    ) -> InitialInteractionResponse {
        InitialInteractionResponse::DeferEphemeralReply
    }

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        close_ticket(ctx, command.user.id, command.channel_id).await?;
        Ok(None)
    }

    async fn handle_component(
        &self,
        ctx: &Context,
        command: &MessageComponentInteraction,
        _args: &[&str],
    ) -> InteractionResult {
        close_ticket(ctx, command.user.id, command.channel_id).await?;
        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

pub async fn close_ticket(ctx: &Context, closer: UserId, channel: ChannelId) -> crate::Result<()> {
    let screenshare = crate::consts::DATABASE
        .fetch_screenshares_for(channel.0)
        .ok_or_else(|| ErrorResponse::message("This channel isn't a screenshare ticket!"))?;

    let mut messages = Vec::new();
    let raw_messages = channel
        .messages_iter(&ctx)
        .boxed()
        .collect::<Vec<_>>()
        .await;

    for message in raw_messages.into_iter().flatten() {
        messages.push(format!(
            "[{}] {}: {}",
            message.timestamp,
            message.author.tag(),
            message.content_safe(ctx)
        ));
        for embed in message.embeds {
            messages.push(format!("Embed:\n{}", FormatEmbed(embed.into())));
        }
    }

    messages.reverse();
    let history = messages.join("\n").into_bytes().into();
    crate::CONFIG
        .ss_logs
        .send_message(&ctx, |msg| {
            msg.files([AttachmentType::Bytes {
                data: history,
                filename: String::from("messages.txt"),
            }])
            .embed(|e| {
                e.title("Screenshare closed").description(format!(
                    "\
                        - Creator: <@{}> \n\
                        - In Question: <@{}> \n\
                        - Closer: <@{}> \
                    ",
                    screenshare.creator, screenshare.in_question, closer
                ))
            })
        })
        .await?;

    crate::consts::DATABASE.remove_entry("Screenshares", channel.0)?;
    channel.delete(&ctx).await?;
    Ok(())
}
