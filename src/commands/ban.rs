use serenity::{
    async_trait,
    builder::CreateEmbed,
    client::{Cache, Context},
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

use crate::{commands::Command, db::BanRoles, interact_opts::InteractOpts};

fn format_db_error(e: &sqlite::Error) -> String {
    if let Some(19) = e.code {
        format!("WARNING: this ban already exists.",)
    } else {
        format!("WARNING: the database responded with an error: {}", e)
    }
}

pub enum BanType {
    Server,
    Scrim,
}

impl BanType {
    pub async fn exec(
        &self,
        http: &Http,
        cache: &Cache,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let user = UserId(command.get_str("user").unwrap().parse()?)
            .to_user(&http)
            .await?;
        let id = user.id;

        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason given."));

        let do_dmd = command.get_bool("dmd").unwrap_or(false);
        let dmd = if do_dmd { 7 } else { 0 };

        let now = OffsetDateTime::now_utc();
        let days = command.get_u64("duration").unwrap_or(30);
        let duration = Duration::from_secs(86400 * days);
        let unban_date = now + duration;

        let mut embed = CreateEmbed::default();
        embed.title(format!("{} recieved a ban", user.tag()));
        embed.field("User", format!("<@{}>", id), false);
        embed.field("Duration", format!("`{} days`", days), false);
        embed.field("Reason", format!("`{}`", reason), false);
        embed.field("Staff", format!("<@{}>", command.user.id), false);

        embed.description("Appeal at https://dyno.gg/form/31ac5763");
        let dm_result = user.dm(&http, |msg| msg.set_embed(embed.clone())).await;
        if let Err(e) = dm_result {
            tracing::error!("Could not DM user {} about their ban: {}", user.tag(), e);
        }
        embed.description("");

        match self {
            Self::Server => {
                let db_result = crate::consts::DATABASE.add_unban(*id.as_u64(), unban_date);
                let result = crate::GUILD
                    .ban_with_reason(&http, id, dmd, reason.clone())
                    .await;

                if result.is_ok() {
                    crate::consts::SUPPORT_BANS
                        .send_message(&http, |msg| msg.set_embed(embed.clone()))
                        .await?;
                }

                command
                    .create_interaction_response(&http, |resp| {
                        resp.interaction_response_data(|data| {
                            match (result.as_ref(), db_result.as_ref()) {
                                (Err(e), _) => {
                                    data.content(format!("Could not ban {}: {}", user.tag(), e))
                                }
                                (Ok(_), Err(e)) => {
                                    embed.description(format_db_error(e));
                                    data.add_embed(embed.clone())
                                }
                                _ => data.add_embed(embed.clone()),
                            }
                        })
                    })
                    .await?;

                result?;
            }
            Self::Scrim => {
                let mut member = crate::GUILD.member(&http, id).await?;

                let mut result = None;
                let mut removed_roles = Vec::new();
                for role in member
                    .roles(&cache)
                    .await
                    .unwrap_or_default()
                    .iter()
                    .filter(|x| !x.managed)
                {
                    dbg!(&role);
                    if let Err(e) = member.remove_role(&http, role).await {
                        if role.id.0 != crate::consts::BOOSTER.0 {
                            let _ = result.get_or_insert(Err(e));
                        }
                    } else {
                        removed_roles.push(role.id);
                        dbg!(&role);
                    }
                }
                result.get_or_insert(member.add_role(&http, crate::consts::BANNED.0).await);

                let db_result = crate::consts::DATABASE.add_scrim_unban(
                    *id.as_u64(),
                    unban_date,
                    BanRoles(removed_roles),
                );
                crate::consts::SUPPORT_BANS
                    .send_message(&http, |msg| msg.set_embed(embed.clone()))
                    .await?;

                command
                    .create_interaction_response(&http, |resp| {
                        resp.interaction_response_data(|data| {
                            match (result.as_ref(), db_result.as_ref()) {
                                (Some(Err(e)), _) => {
                                    data.content(format!("Could not ban {}: {}", user.tag(), e))
                                }
                                (_, Err(e)) => {
                                    embed.description(format_db_error(e));
                                    data.add_embed(embed.clone())
                                }
                                _ => data.add_embed(embed.clone()),
                            }
                        })
                    })
                    .await?;
            }
        }

        Ok(())
    }
}

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
                        o.name("reason")
                            .description("Reason for the ban")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String)
                    })
                    .create_option(|o| {
                        o.name("duration")
                            .description("The ban duration in days")
                            .required(false)
                            .kind(ApplicationCommandOptionType::Integer)
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
        BanType::Server
            .exec(&ctx.http, &ctx.cache, &command)
            .await?;

        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

async fn update_loop(ctx: Arc<Http>) {
    let database = &crate::consts::DATABASE;

    loop {
        let unbans = database.fetch_unbans();
        let now = OffsetDateTime::now_utc();

        for unban in unbans {
            if unban.date < now {
                let _ = crate::GUILD.unban(&ctx, unban.id).await;
                let _ = database.remove_entry("ScheduledUnbans", unban.id);
            }
        }

        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
    }
}

pub struct ScrimBan;
#[async_trait]
impl Command for ScrimBan {
    fn name(&self) -> String {
        String::from("scrimban")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = crate::GUILD
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Screenshare-bans the given user.")
                    .default_permission(false)
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to ban")
                            .required(true)
                            .kind(ApplicationCommandOptionType::User)
                    })
                    .create_option(|o| {
                        o.name("reason")
                            .description("Reason for the ban")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String)
                    })
                    .create_option(|o| {
                        o.name("duration")
                            .description("The ban duration in days")
                            .required(false)
                            .kind(ApplicationCommandOptionType::Integer)
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
                        .id(crate::consts::SS_SUPPORT.0)
                        .permission(true)
                })
            })
            .await?;
        tokio::spawn(scrim_update_loop(ctx.http.clone()));
        Ok(())
    }

    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        BanType::Scrim.exec(&ctx.http, &ctx.cache, &command).await?;

        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

async fn scrim_update_loop(ctx: Arc<Http>) {
    let database = &crate::consts::DATABASE;

    loop {
        let unbans = database.fetch_scrim_unbans();
        let now = OffsetDateTime::now_utc();

        for unban in unbans {
            if unban.date < now {
                let _ = database.remove_entry("ScheduledScrimUnbans", unban.id);
                let member = crate::GUILD.member(&ctx, unban.id).await;

                if let Ok(mut member) = member {
                    let _ = member.add_roles(&ctx, &unban.roles.0).await;
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
    }
}

#[derive(Debug)]
pub enum UnbanType {
    Scrim,
    Server,
}

impl UnbanType {
    pub async fn exec(
        &self,
        http: &Http,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let user = command.get_str("user").unwrap();
        let user_id = UserId(user.parse().unwrap_or_default());
        let bans = crate::GUILD.bans(&http).await?;

        let server_entry = bans
            .iter()
            .find(|x| x.user.id == user_id || user.starts_with(&x.user.tag()));
        let scrim_entry = crate::consts::DATABASE
            .fetch_scrim_unbans()
            .into_iter()
            .find(|x| x.id == user_id.0);
        let exists = match self {
            UnbanType::Scrim => scrim_entry.is_some(),
            UnbanType::Server => server_entry.is_some(),
        };

        if !exists {
            command
                .create_interaction_response(&http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content(format!("Could not find {} in our bans", user))
                    })
                })
                .await?;
            tracing::error!("Could not find {} in {:?}", user, self);
            return Ok(());
        }
        let to_unban = match self {
            UnbanType::Scrim => {
                crate::GUILD
                    .member(&http, scrim_entry.unwrap().id)
                    .await
                    .unwrap()
                    .user
            }
            UnbanType::Server => server_entry.unwrap().user.clone(),
        };

        let mut embed = CreateEmbed::default();
        embed.title(format!("{} was unbanned", to_unban.tag()));
        embed.field("User", format!("<@{}>", to_unban.id), false);
        embed.field("Staff", format!("<@{}>", command.user.id), false);

        match self {
            Self::Server => {
                let result = crate::GUILD.unban(&http, to_unban.id).await;
                let _ = crate::consts::DATABASE.remove_entry("ScheduledUnbans", to_unban.id.0);

                if result.is_ok() {
                    crate::consts::BANS
                        .send_message(&http, |msg| msg.set_embed(embed.clone()))
                        .await?;
                }

                command
                    .create_interaction_response(&http, |resp| {
                        resp.interaction_response_data(|data| {
                            if let Err(ref e) = result.as_ref() {
                                data.content(format!("Could not ban {}: {}", to_unban.tag(), e))
                            } else {
                                data.add_embed(embed)
                            }
                        })
                    })
                    .await?;

                result?;
            }
            Self::Scrim => {
                let mut result = None;
                let mut member = crate::GUILD.member(&http, to_unban.id).await?;
                let unban = crate::consts::DATABASE
                    .fetch_scrim_unbans()
                    .into_iter()
                    .find(|x| x.id == to_unban.id.0)
                    .unwrap();
                if let Err(e) = member.add_roles(&http, &unban.roles.0).await {
                    let _ = result.get_or_insert(Err(e));
                }

                let _ =
                    result.get_or_insert(member.remove_role(&http, crate::consts::BANNED.0).await);

                let db_result =
                    crate::consts::DATABASE.remove_entry("ScheduledScrimUnbans", to_unban.id.0);
                crate::consts::SUPPORT_BANS
                    .send_message(&http, |msg| msg.set_embed(embed.clone()))
                    .await?;

                command
                    .create_interaction_response(&http, |resp| {
                        resp.interaction_response_data(|data| {
                            match (result.as_ref(), db_result.as_ref()) {
                                (Some(Err(e)), _) => {
                                    data.content(format!("Could not ban {}: {}", to_unban.tag(), e))
                                }
                                (_, Err(e)) => {
                                    embed.description(format!(
                                        "WARNING: the database responded with an error: {}",
                                        e
                                    ));
                                    data.add_embed(embed.clone())
                                }
                                _ => data.add_embed(embed.clone()),
                            }
                        })
                    })
                    .await?;
            }
        }

        Ok(())
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
        UnbanType::Server.exec(&ctx.http, &command).await?;
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

pub struct ScrimUnban;

#[async_trait]
impl Command for ScrimUnban {
    fn name(&self) -> String {
        String::from("scrimunban")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = crate::GUILD
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Screenshare-unbans the given user.")
                    .default_permission(false)
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to unban")
                            .required(true)
                            .kind(ApplicationCommandOptionType::User)
                    })
            })
            .await?;
        crate::GUILD
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(crate::consts::SS_SUPPORT.0)
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
        UnbanType::Scrim.exec(&ctx.http, &command).await?;
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

// TODO: list all (scrim)bans
// pub struct Bans;
// pub struct ScrimBans;
