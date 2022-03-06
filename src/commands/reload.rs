use serenity::{
    async_trait,
    client::Context,
    model::interactions::{
        application_command::{ApplicationCommandInteraction, ApplicationCommandPermissionType},
        InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
    },
};

use crate::consts::CONFIG;

pub struct Reload;

#[async_trait]
impl super::Command for Reload {
    fn name(&self) -> String {
        String::from("reload")
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd = CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name())
                    .description("Reloads application commands.")
                    .default_permission(false)
            })
            .await?;

        CONFIG
            .guild
            .create_application_command_permission(&ctx.http, cmd.id, |perm| {
                perm.create_permission(|perm| {
                    perm.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.staff.0)
                        .permission(true)
                })
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
            .create_interaction_response(&ctx.http, |resp| {
                resp.kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        let commands = CONFIG
            .guild
            .get_application_commands(&ctx.http)
            .await?;

        for c in commands {
            tracing::info!("Deleting command {}", c.name);
            CONFIG
                .guild
                .delete_application_command(&ctx.http, c.id)
                .await?;
        }

        command
            .create_followup_message(&ctx.http, |resp| {
                resp.content(
                    "Removed all commands. Please wait for the bot to add the commands back.",
                )
                .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
            })
            .await?;
        Ok(())
    }
    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
