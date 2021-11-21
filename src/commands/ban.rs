use serenity::{
    async_trait,
    client::Context,
    model::{
        id::{RoleId, UserId},
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandOptionType,
            ApplicationCommandPermissionType,
        },
    },
};

use crate::commands::Command;

pub struct Ban {
    perm_role_id: RoleId,
    banned_role_id: RoleId,
}

#[async_trait]
impl Command for Ban {
    fn name(&self) -> String {
        String::from("ban")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = crate::GUILD
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Adds the `Banned` role to the given user")
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to ban")
                            .required(true)
                            .kind(ApplicationCommandOptionType::User)
                    })
            })
            .await?;
        crate::GUILD
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(self.perm_role_id.0)
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
        let to_ban = UserId(
            command.data.options[0]
                .value
                .as_ref()
                .unwrap()
                .to_string()
                .parse()?,
        )
        .to_user(&ctx.http)
        .await?;

        let result = crate::GUILD
            .member(&ctx.http, to_ban.id)
            .await?
            .add_role(&ctx.http, self.banned_role_id)
            .await;

        command
            .create_interaction_response(&ctx.http, |resp| {
                if let Err(ref e) = result.as_ref() {
                    resp.interaction_response_data(|data| {
                        data.content(format!("Could not ban {}: {}", to_ban.name, e))
                    })
                } else {
                    resp.interaction_response_data(|data| {
                        data.content(format!("Successfully banned {}", to_ban.name))
                    })
                }
            })
            .await?;
        result?;
        Ok(())
    }

    fn new() -> Box<Self> {
        let perm_role_id = *crate::consts::SS_SUPPORT;
        let banned_role_id = *crate::consts::BANNED;

        Box::new(Self {
            perm_role_id,
            banned_role_id,
        })
    }
}
