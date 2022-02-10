use serenity::async_trait;
use serenity::client::Context;
use serenity::futures::StreamExt;
use serenity::model::id::UserId;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandOptionType, ApplicationCommandPermissionType,
};
use serenity::model::interactions::InteractionResponseType;
use serenity::model::prelude::InteractionApplicationCommandCallbackDataFlags;

use crate::interact_opts::InteractOpts;

use crate::commands::Command;
use crate::consts::CONFIG;

pub struct Purge;

#[async_trait]
impl Command for Purge {
    fn name(&self) -> String {
        "purge".to_string()
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd = CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Purges a specific amount of messages from the channel")
                    .create_option(|o| {
                        o.name("amount")
                            .description("Amount of messages to purge")
                            .required(true)
                            .kind(ApplicationCommandOptionType::Integer)
                    })
                    .create_option(|o| {
                        o.name("user")
                            .description("When specified, only purges messages from a given user.")
                            .required(false)
                            .kind(ApplicationCommandOptionType::User)
                    })
                    .default_permission(false)
            })
            .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx, cmd.id, |p| {
                for role in &[CONFIG.support, CONFIG.trial_support, CONFIG.staff] {
                    p.create_permission(|perm| {
                        perm.kind(ApplicationCommandPermissionType::Role)
                            .id(role.0)
                            .permission(true)
                    });
                }
                p
            })
            .await?;
        Ok(())
    }

    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        command
            .create_interaction_response(&ctx, |r| {
                r.interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                })
                .kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        let channel = command.channel_id;
        let mut messages = channel.messages_iter(&ctx.http).boxed();

        let max_purge = command.get_i64("amount").unwrap();

        let user_id = command.get_str("user");
        let mut user: Option<UserId> = None;
        if let Some(id) = user_id {
            user = Some(UserId(id.parse()?))
        }
        let mut i = 0;
        while let Some(Ok(message)) = messages.next().await {
            i += 1;

            if i > max_purge {
                break;
            }

            if let Some(u) = user {
                if message.author.id != u {
                    continue;
                }
            }
            message.delete(&ctx.http).await?;
        }
        command
            .edit_original_interaction_response(&ctx.http, |r| {
                r.content("Purge Successfull!".to_string())
            })
            .await?;
        Ok(())
    }

    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Purge {})
    }
}
