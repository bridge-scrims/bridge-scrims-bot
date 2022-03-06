use serenity::{
    async_trait,
    client::Context,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandPermissionType,
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
                })
            })
            .await?;

        Ok(())
    }
    async fn run(
        &self,
        ctx: &Context,
        _command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let commands = CONFIG
            .guild
            .get_application_commands(&ctx.http)
            .await?
            .into_iter()
            .map(|x| x.id);
        for id in commands {
            CONFIG.guild.delete_application_command(&ctx.http, id).await?;
        }
        Ok(())
    }
    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
