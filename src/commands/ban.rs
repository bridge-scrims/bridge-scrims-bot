use std::{sync::Arc, time::Duration};

use serenity::model::application::{
    command::CommandOptionType,
    interaction::{application_command::ApplicationCommandInteraction, MessageFlags},
};
use serenity::model::Permissions;
use serenity::{
    async_trait,
    builder::CreateEmbed,
    client::{Cache, Context},
    http::Http,
    model::id::UserId,
};
use time::OffsetDateTime;

use bridge_scrims::interact_opts::InteractOpts;

use crate::{commands::Command, consts::CONFIG};

use super::unban::{UnbanEntry, UnbanType};

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

        let id = UserId(command.get_str("user").unwrap().parse()?);
        let user_string = match id.to_user(&http).await {
            Ok(user) => user.tag(),
            Err(_) => id.to_string()
        };

        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason given."));

        let duration = command.get_u64("duration");
        let factor = command.get_u64("type");

        let seconds = match (self, duration, factor) {
            (_, Some(d), None) => Some(d * 86400),
            (_, Some(d), Some(f)) => Some(d * f),
            (BanType::Server, None, _) => None,
            (BanType::Scrim, None, _) => Some(30 * 86400),
        };
        let unban_date = seconds.map(|seconds| {
            let now = OffsetDateTime::now_utc();
            let duration = Duration::from_secs(seconds);
            now + duration
        });

        let member = CONFIG.guild.member(&http, id).await;
        if let Ok(ref member) = member {
            let roles = member.roles(&cache).unwrap_or_default();
            let cmd_roles = cmd_member.roles(&cache).unwrap_or_default();
    
            let top_role = roles.iter().max();
            let cmd_top_role = cmd_roles.iter().max();

            if top_role >= cmd_top_role || member.user.bot {
                command
                    .create_interaction_response(&http, |resp| {
                        resp.interaction_response_data(|data| {
                            data.content(format!("You do not have permission to ban {}", user_string))
                                .flags(MessageFlags::EPHEMERAL)
                        })
                    })
                    .await?;
                return Ok(());
            }
        }
        
        let mut embed = CreateEmbed::default();
        embed.title(format!("{} recieved a ban", user_string));
        embed.field("User", format!("<@{}>", id), false);
        if let Some(unban_date) = unban_date {
            embed.field(
                "Duration",
                format!("<t:{}:R>", unban_date.unix_timestamp()),
                false,
            );
        } else {
            embed.field("Duration", "forever", false);
        }
        embed.field("Reason", format!("`{}`", reason), false);
        embed.field("Staff", format!("<@{}>", command.user.id), false);
        if matches!(self, BanType::Server) {
            embed.description("[Click to appeal](https://dyno.gg/form/31ac5763)");
        }
        if let Ok(ref member) = member {
            let _ = member.user.dm(&http, |msg| msg.set_embed(embed.clone())).await;
        }
        embed.description("");
        let mut result = Ok(());
        let mut db_result = Ok(());

        if crate::consts::DATABASE.fetch_freezes_for(id.0).is_some() {
            super::unfreeze::unfreeze_user(http, id).await?;
            command
                .create_followup_message(http, |msg| {
                    msg.content(format!("Unfreezing {} before banning them.", user_string))
                        .flags(MessageFlags::EPHEMERAL)
                })
                .await?;
        }

        match self {
            Self::Server => {
                let do_dmd = command.get_bool("dmd").unwrap_or(false);
                let dmd = if do_dmd { 7 } else { 0 };
                if let Some(unban_date) = unban_date {
                    if crate::consts::DATABASE
                        .fetch_unbans()
                        .iter()
                        .any(|x| x.id == id.0)
                    {
                        embed.title(format!("{}'s ban has been modified", user_string));
                        db_result = crate::consts::DATABASE.modify_unban_date(
                            "ScheduledUnbans",
                            *id.as_u64(),
                            unban_date,
                        );
                    } else {
                        db_result = crate::consts::DATABASE.add_unban(*id.as_u64(), unban_date);
                    }
                }
                result = CONFIG
                    .guild
                    .ban_with_reason(&http, id, dmd, reason.clone())
                    .await;

                if result.is_ok() {
                    CONFIG
                        .support_bans
                        .send_message(&http, |msg| msg.set_embed(embed.clone()))
                        .await?;
                }
            }
            Self::Scrim => {
                if crate::consts::DATABASE
                    .fetch_scrim_unbans()
                    .iter()
                    .any(|x| x.id == id.0)
                {
                    embed.title(format!("{}'s ban has been modified", user_string));
                    db_result = crate::consts::DATABASE.modify_unban_date(
                        "ScheduledScrimUnbans",
                        *id.as_u64(),
                        unban_date.unwrap(),
                    );
                } else {
                    let mut removed_roles = Vec::new();
                    if let Ok(mut member) = member {
                        let roles = member.roles(&cache).unwrap_or_default();
                        for role in roles.iter().filter(|x| !x.managed) {
                            if let Err(e) = member.remove_role(&http, role).await {
                                result = result.and(Err(e));
                            } else {
                                removed_roles.push(role.id);
                            }
                        }
                        result = result.and(member.add_role(&http, CONFIG.banned.0).await);
                    }
                    
                    db_result = crate::consts::DATABASE.add_scrim_unban(
                        *id.as_u64(),
                        // NOTE: In the case of a `ScrimBan`, this is always `Some`
                        unban_date.unwrap(),
                        &removed_roles.into(),
                    );
                }

                CONFIG
                    .support_bans
                    .send_message(&http, |msg| msg.set_embed(embed.clone()))
                    .await?;
            }
        }

        if let Err(ref e) = result {
            command
                .create_interaction_response(&http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content(format!("Could not ban {}: {}", user_string, e))
                            .flags(MessageFlags::EPHEMERAL)
                    })
                })
                .await?;
        } else if command.channel_id != CONFIG.support_bans {
            command
                .create_interaction_response(&http, |resp| {
                    resp.interaction_response_data(|data| {
                        match (result.as_ref(), db_result.as_ref()) {
                            (Ok(_), Err(e)) => {
                                embed.description(format_db_error(e));
                                data.add_embed(embed.clone())
                            }
                            _ => data.add_embed(embed.clone()),
                        }
                    })
                })
                .await?;
        }

        result?;

        Ok(())
    }
}

pub struct Ban;

#[async_trait]
impl Command for Ban {
    fn name(&self) -> String {
        String::from("ban")
    }
    async fn init(&self, ctx: &Context) {
        tokio::spawn(update_loop(ctx.http.clone()));
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG.guild
            .create_application_command(&ctx, |c| {
                c
                    .name(self.name())
                    .description("Bans the given user from the server. This is not meant for screenshare bans.")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to ban")
                            .required(true)
                            .kind(CommandOptionType::User)
                    })
                    .create_option(|o| {
                        o.name("reason")
                            .description("Reason for the ban")
                            .required(true)
                            .kind(CommandOptionType::String)
                    })
                    .create_option(|o| {
                        o.name("duration")
                            .description("The ban duration. Default: forever")
                            .required(false)
                            .kind(CommandOptionType::Integer)
                    })
                    .create_option(|o| {
                        o.name("type")
                            .description("The ban duration type. Default: Days")
                            .required(false)
                            .kind(CommandOptionType::Integer)
                            .add_int_choice("Seconds", 1)
                            .add_int_choice("Minutes", 60)
                            .add_int_choice("Hours", 60 * 60)
                            .add_int_choice("Days", 60 * 60 * 24)
                    })
                    .create_option(|o| {
                        o.name("dmd")
                            .description("Should the last 7d of messages be removed?")
                            .required(false)
                            .kind(CommandOptionType::Boolean)
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
    async fn init(&self, ctx: &Context) {
        tokio::spawn(scrim_update_loop(ctx.http.clone()));
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Screenshare-bans the given user.")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to ban")
                            .required(true)
                            .kind(CommandOptionType::User)
                    })
                    .create_option(|o| {
                        o.name("reason")
                            .description("Reason for the ban")
                            .required(true)
                            .kind(CommandOptionType::String)
                    })
                    .create_option(|o| {
                        o.name("duration")
                            .description("The ban duration. Default: 30")
                            .required(false)
                            .kind(CommandOptionType::Integer)
                    })
                    .create_option(|o| {
                        o.name("type")
                            .description("The ban duration type. Default: days")
                            .required(false)
                            .kind(CommandOptionType::Integer)
                            .add_int_choice("Seconds", 1)
                            .add_int_choice("Minutes", 60)
                            .add_int_choice("Hours", 60 * 60)
                            .add_int_choice("Days", 60 * 60 * 24)
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

// TODO: list all (scrim)bans
// pub struct Bans;
// pub struct ScrimBans;
