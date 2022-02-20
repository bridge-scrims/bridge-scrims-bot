use bridge_scrims::interact_opts::InteractOpts;
use serenity::{
    async_trait,
    client::Context,
    model::{
        channel::{PermissionOverwrite, PermissionOverwriteType},
        id::UserId,
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandOptionType,
        },
    },
};

use super::Command;

pub struct Ticket;

#[async_trait]
impl Command for Ticket {
    fn name(&self) -> String {
        String::from("ticket")
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
                            .kind(ApplicationCommandOptionType::String)
                            .add_string_choice("Add", "a")
                            .add_string_choice("Remove", "r")
                    })
                    .create_option(|opt| {
                        opt.name("target")
                            .description("The user that is affected by the change")
                            .kind(ApplicationCommandOptionType::User)
                            .required(true)
                    })
            })
            .await?;
        // TODO: Permissions
        Ok(())
    }
    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let who = UserId(command.get_str("target").unwrap().parse()?);
        let operation = command.get_str("operation").unwrap();
        let channel = command
            .channel_id
            .to_channel(&ctx.http)
            .await?
            .guild()
            .unwrap();
        match operation.as_str() {
            "a" => {
                channel
                    .create_permission(
                        &ctx.http,
                        &PermissionOverwrite {
                            allow: super::screenshare::ALLOW_PERMS.clone(),
                            deny: super::screenshare::DENY_PERMS.clone(),
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
                        })
                    })
                    .await?;
                return Ok(());
            }
        }
        Ok(())
    }
    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
