use serenity::{
    async_trait,
    builder::CreateEmbed,
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
use std::{sync::Arc, time::Duration};
use time::OffsetDateTime;

use crate::{commands::Command, interact_opts::InteractOpts};

pub struct Ban;
#[async_trait]
impl Command for Ban {
    fn name(&self) -> String {
        String::from("ban")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = crate::GUILD
            .create_application_command(&ctx, |c| {
                c
                    .name(self.name())
                    .description("Bans the given user from the server. This is not meant for screenshare bans.")
                    .default_permission(false)
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
                        .id(crate::consts::SUPPORT.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(crate::consts::STAFF.0)
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
        let to_ban = UserId(command.get_str("user").unwrap().parse()?)
            .to_user(&ctx.http)
            .await?;

        let now = OffsetDateTime::now_utc();
        let amount_days = command.get_u64("duration").unwrap_or(30);
        let duration = Duration::from_secs(86400 * amount_days);
        let unban_date = now + duration;

        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason given."));

        let do_dmd = command.get_bool("dmd").unwrap_or(false);
        let dmd = if do_dmd { 7 } else { 0 };

        let mut embed = CreateEmbed::default();
        embed.title(format!("{} recieved a ban", to_ban.tag()));
        embed.field("User", format!("<@{}>", to_ban.id), false);
        embed.field("Duration", format!("`{} days`", amount_days), false);
        embed.field("Reason", format!("`{}`", reason), false);
        embed.field("Staff", format!("<@{}>", command.user.id), false);

        embed.description("Appeal at https://dyno.gg/form/31ac5763");
        let dm_result = to_ban.dm(&ctx.http, |msg| msg.set_embed(embed.clone())).await;
        if let Err(e) = dm_result {
            tracing::error!("Could not DM user {} about their ban: {}", to_ban.tag(), e);
        }
        embed.description("");

        let result = crate::GUILD
            .ban_with_reason(&ctx.http, to_ban.id, dmd, reason.clone())
            .await;
        let db_result = crate::consts::DATABASE.add_unban(*to_ban.id.as_u64(), unban_date);

        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.interaction_response_data(|data| match (result.as_ref(), db_result.as_ref()) {
                    (Err(e), _) => data.content(format!("Could not ban {}: {}", to_ban.tag(), e)),
                    (Ok(_), Err(e)) => {
                        embed.description(format!(
                            "WARNING: the database responded with an error: {}",
                            e
                        ));
                        data.add_embed(embed.clone())
                    }
                    _ => data.add_embed(embed.clone()),
                })
            })
            .await?;

        if result.is_ok() {
            crate::consts::SUPPORT_BANS
                .send_message(&ctx.http, |msg| msg.set_embed(embed.clone()))
                .await?;
        }

        result?;
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

async fn update_loop(ctx: Arc<Http>) {
    let database = &crate::consts::DATABASE;

    loop {
        let unbans = database.fetch_unbans().await;
        let now = OffsetDateTime::now_utc();

        for unban in unbans {
            if unban.1 < now {
                let _ = crate::GUILD.unban(&ctx, unban.0).await;
                let _ = database.remove_unban(unban.0);
            }
        }

        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
    }
}

pub struct Unban;

#[async_trait]
impl Command for Unban {
    fn name(&self) -> String {
        String::from("unban")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = crate::GUILD
            .create_application_command(&ctx, |c| {
                c
                    .name(self.name())
                    .description("Removes a ban from the given user in this server. This does not affect the \"Banned\" Role")
                    .default_permission(false)
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to unban")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String)
                    })
            })
        .await?;
        crate::GUILD
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(crate::consts::SUPPORT.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(crate::consts::STAFF.0)
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
        let user = command.get_str("user").unwrap();
        let user_id = UserId(user.parse().unwrap_or_default());
        let bans = crate::GUILD.bans(&ctx.http).await?;
        let to_unban = &match bans.iter().find(|x| {
            x.user.id == user_id || user.starts_with(&x.user.tag())
        }) {
            Some(u) => u,
            None => {
                command
                    .create_interaction_response(&ctx.http, |resp| {
                        resp.interaction_response_data(|data| {
                            data.content(format!("Could not find {} in your bans", user))
                        })
                    })
                    .await?;
                return Ok(());
            }
        }
        .user;

        let result = crate::GUILD.unban(&ctx.http, to_unban.id).await;
        let _ = crate::consts::DATABASE.remove_unban(to_unban.id.0);

        let mut embed = CreateEmbed::default();
        embed.title(format!("{} was unbanned", to_unban.tag()));
        embed.field("User", format!("<@{}>", to_unban.id), false);
        embed.field("Staff", format!("<@{}>", command.user.id), false);

        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.interaction_response_data(|data| {
                    if let Err(ref e) = result.as_ref() {
                        data.content(format!("Could not unban {}: {}", to_unban.tag(), e))
                    } else {
                        data.add_embed(embed.clone())
                    }
                })
            })
            .await?;

        if result.is_ok() {
            crate::consts::SUPPORT_BANS
                .send_message(&ctx.http, |msg| msg.set_embed(embed))
                .await?;
        }

        result?;
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
