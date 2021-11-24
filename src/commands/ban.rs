use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    client::Context,
    http::Http,
    model::{
        id::{RoleId, UserId},
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandOptionType,
            ApplicationCommandPermissionType,
        },
    },
};
use std::{sync::Arc, time::Duration};
use time::OffsetDateTime;

use crate::{commands::Command, db::Database};

fn ban_opts(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .create_option(|o| {
            o.name("user")
                .description("The user to ban")
                .required(true)
                .kind(ApplicationCommandOptionType::User)
        })
        .create_option(|o| {
            o.name("duration")
                .description("The ban duration in days")
                .required(false)
                .kind(ApplicationCommandOptionType::Integer)
        })
        .create_option(|o| {
            o.name("reason")
                .description("Reason for the ban")
                .required(false)
                .kind(ApplicationCommandOptionType::String)
        })
}

pub struct Ban {
    pub support_role_id: RoleId,
    pub staff_role_id: RoleId,
    pub database: Database,
}

#[async_trait]
impl Command for Ban {
    fn name(&self) -> String {
        String::from("ban")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = crate::GUILD
            .create_application_command(&ctx, |c| {
                ban_opts(c.name(self.name()).description(
                    "Bans the given user from the server. This is not meant for screenshare bans.",
                ).default_permission(false))
                .create_option(|o| {
                    o.name("dmd")
                        .description("Should the last 7d of messages be removed?")
                        .required(false)
                        .kind(ApplicationCommandOptionType::Boolean)
                })
            })
            .await?;
        crate::GUILD
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(self.support_role_id.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(self.staff_role_id.0)
                        .permission(true)
                })
            })
            .await?;
        tokio::spawn(update_loop(ctx.http.clone()));
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
                .as_str()
                .unwrap()
                .parse()?,
        )
        .to_user(&ctx.http)
        .await?;

        let now = OffsetDateTime::now_utc();
        let duration = Duration::from_secs(
            86400 *
            command
                .data
                .options
                .iter()
                .find(|x| x.name.as_str() == "duration")
                .map_or(Some(30), |x| x.value.as_ref().map(|x| x.as_u64().unwrap()))
                .unwrap(),
        );
        let unban_date = now + duration;
        let reason = command
            .data
            .options
            .iter()
            .find(|x| x.name.as_str() == "reason")
            .map(|x| x.value.as_ref().unwrap().to_string())
            .unwrap_or_default();
        let do_dmd = command
            .data
            .options
            .iter()
            .find(|x| x.name.as_str() == "dmd")
            .map_or(Some(false), |x| {
                x.value.as_ref().map(|x| x.as_bool().unwrap())
            })
            .unwrap();
        let dmd = if do_dmd { 7 } else { 0 };

        let result = crate::GUILD
            .ban_with_reason(&ctx.http, to_ban.id, dmd, reason)
            .await;
        self.database.add_unban(*to_ban.id.as_u64(), unban_date);

        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.interaction_response_data(|data| {
                    if let Err(ref e) = result.as_ref() {
                        data.content(format!("Could not ban {}: {}", to_ban.tag(), e))
                    } else {
                        // TODO: better message, see notes
                        data.create_embed(|em| {
                            em.description(format!("Successfully banned {}", to_ban.tag()))
                        })
                    }
                })
            })
            .await?;
        result?;
        Ok(())
    }

    fn new() -> Box<Self> {
        let support_role_id = *crate::consts::SUPPORT;
        let staff_role_id = *crate::consts::STAFF;
        let database = Database::init();

        Box::new(Self {
            support_role_id,
            staff_role_id,
            database,
        })
    }
}

async fn update_loop(ctx: Arc<Http>) {
    let mut database = Database::init();

    loop {
        database.fetch_unbans().await;
        let now = OffsetDateTime::now_utc();

        for unban in database.cache.0.iter() {
            if unban.1 < now {
                let _ = crate::GUILD.unban(&ctx, unban.0).await;
                database.remove_unban(unban.0);
            }
        }

        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
    }
}
