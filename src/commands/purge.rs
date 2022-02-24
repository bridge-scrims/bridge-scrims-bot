use std::collections::HashMap;

use crate::interact_opts::InteractOpts;
use serenity::async_trait;
use serenity::builder::CreateApplicationCommand;
use serenity::client::Context;
use serenity::futures::StreamExt;
use serenity::model::channel::Message;

use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
    ApplicationCommandOptionType, ApplicationCommandPermissionType,
};
use serenity::model::interactions::InteractionResponseType;
use serenity::model::prelude::InteractionApplicationCommandCallbackDataFlags;

use crate::commands::Command;
use crate::consts::CONFIG;

#[async_trait]
pub trait PurgeOption {
    fn name(&self) -> String;
    fn register(&self, cmd: &mut CreateApplicationCommand);
    async fn check(&self, subcmd: &ApplicationCommandInteractionDataOption, msg: Message) -> bool;
    fn new() -> Box<Self>
    where
        Self: Sized;
}

pub struct Purge {
    options: HashMap<String, Box<dyn PurgeOption + Send + Sync>>,
}

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
                    .default_permission(false);
                for option in self.options.values() {
                    option.register(c);
                }
                c
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
        let cmd = &command.data.options[0];
        let max_purge = cmd.get_i64("amount").unwrap_or(50);

        let option = self.options.get(&cmd.name).unwrap();
        let mut i = 0;
        while let Some(Ok(message)) = messages.next().await {
            i += 1;

            if i > max_purge {
                break;
            }

            if option.check(cmd, message.clone()).await {
                // ignore errors here since it doesn't matter if we can't delete
                let _ = message.delete(&ctx.http).await;
            }
        }
        command
            .edit_original_interaction_response(&ctx.http, |r| {
                r.content("Purge Successful!".to_string())
            })
            .await?;
        Ok(())
    }

    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        let options: Vec<Box<dyn PurgeOption + Send + Sync>> = vec![All::new(), FromUser::new()];
        let options = options.into_iter().fold(
            HashMap::new(),
            |mut map, opt: Box<dyn PurgeOption + Send + Sync>| {
                map.insert(opt.name(), opt);
                map
            },
        );

        Box::new(Purge { options })
    }
}

struct All;

#[async_trait]
impl PurgeOption for All {
    fn name(&self) -> String {
        "all".to_string()
    }
    fn register(&self, cmd: &mut CreateApplicationCommand) {
        cmd.create_option(|opt| {
            opt.name(self.name())
                .kind(ApplicationCommandOptionType::SubCommand)
                .description("Purges a certain amount of messages in the current channel.")
                .create_sub_option(|amount| {
                    amount
                        .name("amount")
                        .description("The amount of messages to purge.")
                        .kind(ApplicationCommandOptionType::Integer)
                        .required(true)
                })
        });
    }
    async fn check(
        &self,
        _subcmd: &ApplicationCommandInteractionDataOption,
        _msg: Message,
    ) -> bool {
        true
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(All {})
    }
}

struct FromUser;

#[async_trait]
impl PurgeOption for FromUser {
    fn name(&self) -> String {
        "from_user".to_string()
    }
    fn register(&self, cmd: &mut CreateApplicationCommand) {
        cmd.create_option(|opt| {
            opt.name(self.name())
                .kind(ApplicationCommandOptionType::SubCommand)
                .description("Purges a certain amount of messages in the current channel.")
                .create_sub_option(|amount| {
                    amount
                        .name("amount")
                        .kind(ApplicationCommandOptionType::Integer)
                        .description(
                            "The amount of messages to go through to purge (total messages)",
                        )
                        .required(true)
                })
                .create_sub_option(|opt| {
                    opt.name("user")
                        .kind(ApplicationCommandOptionType::User)
                        .required(true)
                        .description("The user who's messages are to be purged.")
                })
        });
    }
    async fn check(&self, subcmd: &ApplicationCommandInteractionDataOption, msg: Message) -> bool {
        let x: u64 = subcmd.get_str("user").unwrap().parse().unwrap();
        msg.author.id.0 == x
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(FromUser {})
    }
}
