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

use crate::{commands::Command, consts::CONFIG};
use bridge_scrims::interact_opts::InteractOpts;

fn format_db_error(e: &sqlite::Error) -> String {
    if let Some(19) = e.code {
        "WARNING: this ban already exists.".to_string()
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
        let cmd_member = command.clone().member.unwrap();

        let user = UserId(command.get_str("user").unwrap().parse()?)
            .to_user(&http)
            .await?;
        let id = user.id;

        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason given."));

        let seconds = command.get_u64("duration");
        let seconds = match (self, seconds) {
            (_, Some(d)) => Some(d),
            (BanType::Server, None) => None,
            (BanType::Scrim, None) => Some(30 * 86400),
        };
        let unban_date = seconds.map(|seconds| {
            let now = OffsetDateTime::now_utc();
            let duration = Duration::from_secs(seconds);
            now + duration
        });

        let mut member = CONFIG.guild.member(&http, id).await?;
        let roles = member.roles(&cache).await.unwrap_or_default();
        let cmd_roles = cmd_member.roles(&cache).await.unwrap_or_default();

        let top_role = roles.iter().max();
        let cmd_top_role = cmd_roles.iter().max();

        if top_role >= cmd_top_role || user.bot {
            command
                .create_interaction_response(&http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content(format!("You do not have permission to ban {}", user.tag()))
                    })
                })
                .await?;
            return Ok(());
        }

        let mut embed = CreateEmbed::default();
        embed.title(format!("{} recieved a ban", user.tag()));
        embed.field("User", format!("<@{}>", id), false);
        if let Some(unban_date) = unban_date {
            embed.field("Duration", format!("<t:{}:R>", unban_date.unix_timestamp()), false);
        } else {
            embed.field("Duration", "forever", false);
        }
        embed.field("Reason", format!("`{}`", reason), false);
        embed.field("Staff", format!("<@{}>", command.user.id), false);
        if matches!(self, BanType::Server) {
            embed.description("Appeal at http://appeal.bridgescrims.com/");
        }
        let dm_result = user.dm(&http, |msg| msg.set_embed(embed.clone())).await;
        if let Err(e) = dm_result {
            tracing::error!("Could not DM user {} about their ban: {}", user.tag(), e);
        }
        embed.description("");
        match self {
            Self::Server => {
                let do_dmd = command.get_bool("dmd").unwrap_or(false);
                let dmd = if do_dmd { 7 } else { 0 };
                let db_result = if let Some(unban_date) = unban_date {
                    crate::consts::DATABASE.add_unban(*id.as_u64(), unban_date)
                } else {
                    Ok(())
                };
                let result = CONFIG
                    .guild
                    .ban_with_reason(&http, id, dmd, reason.clone())
                    .await;

                if result.is_ok() {
                    CONFIG
                        .support_bans
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
                let mut result = None;
                let mut removed_roles = Vec::new();
                for role in roles.iter().filter(|x| !x.managed) {
                    if let Err(e) = member.remove_role(&http, role).await {
                        let _ = result.get_or_insert(Err(e));
                    } else {
                        removed_roles.push(role.id);
                    }
                }
                result.get_or_insert(member.add_role(&http, CONFIG.banned.0).await);

                let db_result = crate::consts::DATABASE.add_scrim_unban(
                    *id.as_u64(),
                    // NOTE: In the case of a `ScrimBan`, this is always `Some`
                    unban_date.unwrap(),
                    &removed_roles.into(),
                );
                CONFIG
                    .support_bans
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
        let command = CONFIG.guild
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
        CONFIG
            .guild
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.staff.0)
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
        BanType::Server.exec(&ctx.http, &ctx.cache, command).await?;

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
                let _ = CONFIG
                    .guild
                    .unban(&ctx, unban.id)
                    .await
                    .map(|_| database.remove_entry("ScheduledUnbans", unban.id));
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
        let command = CONFIG
            .guild
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
                            .description("The ban duration")
                            .required(false)
                            .kind(ApplicationCommandOptionType::Integer)
                            .add_int_choice("Seconds", 1)
                            .add_int_choice("Minutes", 60)
                            .add_int_choice("Hours", 60 * 60)
                            .add_int_choice("Days", 60 * 60 * 24)
                    })
            })
            .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.ss_support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.staff.0)
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
        BanType::Scrim.exec(&ctx.http, &ctx.cache, command).await?;

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
            if unban.date > now {
                continue;
            }
            let member = CONFIG.guild.member(&ctx, unban.id).await;
            if let Ok(member) = member {
                let res = UnbanType::Scrim
                    .unban(
                        &ctx,
                        None,
                        UnbanEntry::Scrim(unban),
                        "Ban Expired".to_string(),
                    )
                    .await;
                if let Err(err) = res {
                    tracing::error!("Failed to unban {} upon expiration: {}", member, err);
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

pub enum UnbanEntry {
    Scrim(crate::model::ScrimUnban),
    Server(serenity::model::guild::Ban),
}

impl UnbanType {
    pub async fn exec(
        &self,
        http: &Http,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let user = command.get_str("user").unwrap();
        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason given."));
        let user_id = UserId(user.parse().unwrap_or_default());
        let bans = CONFIG.guild.bans(&http).await?;

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

        let entry = match self {
            UnbanType::Scrim => UnbanEntry::Scrim(scrim_entry.unwrap()),
            UnbanType::Server => UnbanEntry::Server(server_entry.unwrap().clone()),
        };
        async move {
            let res = self.unban(http, Some(command.user.id), entry, reason).await;
            let _ = command
                .create_interaction_response(&http, |resp| {
                    resp.interaction_response_data(|data| match res {
                        Err(err) => data.content(format!("Could not unban {}: {}", user_id, err)),
                        Ok(embed) => data.embeds([embed]),
                    })
                })
                .await;
        }
        .await;
        Ok(())
    }
    pub async fn unban(
        &self,
        http: &Http,
        staff_id: Option<UserId>, // If staff id is not provided, it is assumed that the ban expired
        unban_entry: UnbanEntry,
        reason: String,
    ) -> Result<CreateEmbed, Box<dyn std::error::Error + Sync + Send>> {
        let to_unban = match unban_entry {
            UnbanEntry::Scrim(entry) => CONFIG.guild.member(&http, entry.id).await.unwrap().user,
            UnbanEntry::Server(entry) => entry.user.clone(),
        };

        let mut embed = CreateEmbed::default();
        embed.title(format!("{} was unbanned", to_unban.tag()));
        embed.field("User", format!("<@{}>", to_unban.id), false);
        if staff_id.is_some() {
            embed.field("Staff", format!("<@{}>", staff_id.unwrap()), false);
        }

        embed.field("Reason", format!("`{}`", reason), false);

        match self {
            Self::Server => {
                let result = CONFIG.guild.unban(&http, to_unban.id).await;
                if result.is_ok() {
                    crate::consts::DATABASE
                        .remove_entry("ScheduledUnbans", to_unban.id.0)
                        .unwrap_or_else(|_| {
                            tracing::error!(
                                "Could not remove {} from the ban database.",
                                to_unban.tag()
                            )
                        });
                }
                match result {
                    Ok(_) => {
                        let _ = CONFIG
                            .support_bans
                            .send_message(&http, |msg| msg.set_embed(embed.clone()))
                            .await;
                        return Ok(embed);
                    }
                    Err(err) => return Err(Box::new(err)),
                }
            }
            Self::Scrim => {
                let mut result = None;
                let mut member = match CONFIG.guild.member(&http, to_unban.id).await {
                    Ok(memb) => memb,
                    Err(e) => return Err(Box::new(e)),
                };
                let unban = crate::consts::DATABASE
                    .fetch_scrim_unbans()
                    .into_iter()
                    .find(|x| x.id == to_unban.id.0)
                    .unwrap();
                let roles: Vec<_> = unban.roles.into();
                if let Err(e) = member.add_roles(&http, &roles).await {
                    return Err(Box::new(e)); // If roles cannot be added, don't remove the unban from the database either
                }

                let _ = result.get_or_insert(member.remove_role(&http, CONFIG.banned.0).await);

                let db_result =
                    crate::consts::DATABASE.remove_entry("ScheduledScrimUnbans", to_unban.id.0);
                let _ = CONFIG
                    .support_bans
                    .send_message(&http, |msg| msg.set_embed(embed.clone()))
                    .await;
                if let Err(e) = db_result.as_ref() {
                    embed.description(format!(
                        "WARNING: the database responded with an error: {}",
                        e
                    ));
                }
            }
        }
        Ok(embed)
    }
}

pub struct Unban;

#[async_trait]
impl Command for Unban {
    fn name(&self) -> String {
        String::from("unban")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = CONFIG.guild
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
                    .create_option(|o| {
                        o.name("reason")
                            .description("The reason to remove this user's ban")
                            .kind(ApplicationCommandOptionType::String)
                            .required(false)
                    })

            })
        .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
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
        UnbanType::Server.exec(&ctx.http, command).await?;
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
        let command = CONFIG
            .guild
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
                    .create_option(|o| {
                        o.name("reason")
                            .description("The reason to remove this user's ban")
                            .kind(ApplicationCommandOptionType::String)
                            .required(false)
                    })
            })
            .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.ss_support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
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
        UnbanType::Scrim.exec(&ctx.http, command).await?;
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

// TODO: list all (scrim)bans
// pub struct Bans;
// pub struct ScrimBans;
