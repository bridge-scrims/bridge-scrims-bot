use serenity::{
    async_trait,
    client::Context,
    http::Http,
    model::{
        id::UserId,
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandOptionType,
            ApplicationCommandPermissionType,
        },
    },
};

use bridge_scrims::interact_opts::InteractOpts;

use super::Command;

pub struct Unfreeze;

#[async_trait]
impl Command for Unfreeze {
    fn name(&self) -> String {
        String::from("unfreeze")
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = crate::CONFIG
            .guild
            .create_application_command(&ctx.http, |command| {
                command
                    .name(self.name())
                    .description("Unfreezes a user")
                    .default_permission(false)
                    .create_option(|opt| {
                        opt.name("player")
                            .description("The player to unfreeze")
                            .kind(ApplicationCommandOptionType::User)
                            .required(true)
                    })
            })
            .await?;
        crate::CONFIG
            .guild
            .create_application_command_permission(&ctx, command.id, |p| {
                p.create_permission(|perm| {
                    perm.kind(ApplicationCommandPermissionType::Role)
                        .id(crate::CONFIG.ss_support.0)
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
        let emoji = crate::CONFIG
            .guild
            .emoji(&ctx.http, crate::CONFIG.unfreeze_emoji)
            .await?;
        let user = UserId(command.get_str("player").unwrap().parse()?);
        let unfreeze = unfreeze_user(&ctx.http, user).await?;
        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.interaction_response_data(|data| {
                    if !unfreeze {
                        data.create_embed(|embed| {
                            embed
                                .title("Not frozen")
                                .description(format!("{} is not frozen", user))
                        })
                    } else {
                        if let Some(mut sc) =
                            crate::consts::DATABASE.get_screensharer(command.user.id.0)
                        {
                            sc.freezes += 1;
                            let _ = crate::consts::DATABASE.set_screensharer(sc);
                        } else {
                            let _ = crate::consts::DATABASE.set_screensharer(crate::db::Screensharer {
                                id: command.user.id.0,
                                freezes: 1,
                            });
                        }
                        data.content(format!("{} <@{}>, you are now unfrozen", emoji, user))
                    }
                })
            })
            .await?;
        Ok(())
    }
    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

pub async fn unfreeze_user(http: &Http, user: UserId) -> crate::Result<bool> {
    let freeze = crate::consts::DATABASE.fetch_freezes_for(user.0);
    if freeze.is_none() {
        return Ok(false);
    }
    let freeze = freeze.unwrap();

    let mut member = crate::CONFIG.guild.member(&http, user).await?;
    member.remove_role(&http, crate::CONFIG.frozen).await?;
    member.add_roles(&http, &freeze.roles).await?;
    crate::consts::DATABASE.remove_entry("Freezes", user.0)?;
    Ok(true)
}
