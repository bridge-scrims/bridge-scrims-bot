use serenity::{
    async_trait, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::prelude::*,
};

use crate::consts::DATABASE;
use bridge_scrims::interaction::*;

pub struct Ticket;

#[async_trait]
impl InteractionHandler for Ticket {
    fn name(&self) -> String {
        String::from("ticket")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec![crate::CONFIG.ss_support])
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name())
                    .description("Adds/removes someone to an existing ticket")
                    .create_option(|opt| {
                        opt.name("operation")
                            .description("Wether to add or remove someone")
                            .required(true)
                            .kind(command::CommandOptionType::String)
                            .add_string_choice("Add", "a")
                            .add_string_choice("Remove", "r")
                    })
                    .create_option(|opt| {
                        opt.name("target")
                            .description("The user that is affected by the change")
                            .kind(command::CommandOptionType::User)
                            .required(true)
                    })
                    .default_member_permissions(Permissions::empty())
            })
            .await?;
        Ok(())
    }

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        let who = UserId(command.get_str("target").unwrap().parse()?);
        let operation = command.get_str("operation").unwrap();
        let channel = command
            .channel_id
            .to_channel(&ctx.http)
            .await?
            .guild()
            .unwrap();

        if DATABASE
            .fetch_screenshares_for(channel.id.0)
            .await?
            .is_none()
        {
            command
                .create_interaction_response(&ctx.http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content("That channel is not a ticket!")
                            .flags(interaction::MessageFlags::EPHEMERAL)
                    })
                })
                .await?;

            return Ok(None);
        }

        match operation.as_str() {
            "a" => {
                channel
                    .create_permission(
                        &ctx.http,
                        &PermissionOverwrite {
                            allow: *super::screenshare::ALLOW_PERMS,
                            deny: *super::screenshare::DENY_PERMS,
                            kind: PermissionOverwriteType::Member(who),
                        },
                    )
                    .await?;
                command
                    .create_interaction_response(&ctx.http, |resp| {
                        resp.interaction_response_data(|data| {
                            data.content(format!("<@{}> has been added to the ticket.", who))
                        })
                    })
                    .await?;
            }
            "r" => {
                channel
                    .delete_permission(&ctx.http, PermissionOverwriteType::Member(who))
                    .await?;
                command
                    .create_interaction_response(&ctx.http, |resp| {
                        resp.interaction_response_data(|data| {
                            data.content(format!("<@{}> has been removed from the ticket.", who))
                        })
                    })
                    .await?;
            }
            _ => {
                command
                    .create_interaction_response(&ctx.http, |resp| {
                        resp.interaction_response_data(|data| {
                            data.content("That is not an option.")
                                .flags(interaction::MessageFlags::EPHEMERAL)
                        })
                    })
                    .await?;
                return Ok(None);
            }
        }
        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
