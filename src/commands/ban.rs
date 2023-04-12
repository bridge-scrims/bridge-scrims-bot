use std::{time::Duration, collections::HashSet};
use time::OffsetDateTime;

use serenity::{
    async_trait,
    client::Context,
    builder::{CreateEmbed, CreateInteractionResponseData, CreateEmbedAuthor},

    model::prelude::*,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction,
        MessageFlags
    }
};

use bridge_scrims::{
    parse_durations::Duration as ParsedDuration,
    interaction::*
};

use crate::{consts::CONFIG, db::Ids};

pub enum BanType {
    Server,
    Scrim,
}

fn parse_duration<'a>(resolvable: String) -> err_resp::Result<'a, ParsedDuration> {
    let duration = resolvable.parse::<ParsedDuration>();
    if duration.is_err() {
        return Err(ErrorResponse::with_title("Invalid Duration", "Please provide a valid ban duration (e.g. 5m 30d)."))?;
    }
    Ok(duration.unwrap())
}

async fn send_ban_log(ctx: &Context, embed: CreateEmbed) {
    let _ = CONFIG.support_bans
        .send_message(&ctx, |msg| msg.set_embed(embed.clone())).await
        .map_err(|e| tracing::error!("Failed to log to support_bans: {}", e));
}

impl BanType {

    fn get_name(&self) -> String {
        match self {
            Self::Server => String::from("Server"),
            Self::Scrim => String::from("Scrim")
        }
    }

    async fn ban(
        &self, ctx: &Context, user: User, member: Option<Member>, executor: UserId, 
        unban_date: Option<OffsetDateTime>, dmd: u8, reason: String
    ) -> crate::Result<CreateEmbed> {
        
        let user_id = user.id;
        let mut author = CreateEmbedAuthor::default();
        author.icon_url(user.avatar_url().unwrap_or_else(|| user.default_avatar_url()));

        let mut fields = Vec::new();
        match unban_date {
            Some(v) => fields.push(("Expires", format!("<t:{}:R>", v.unix_timestamp()), false)),
            None => fields.push(("Expires", "never".to_string(), false))
        };

        fields.push(("Staff", executor.mention().to_string(), false));
        fields.push(("Reason", format!("`{}`", reason), false));
        
        let mut embed = CreateEmbed::default();
        embed
            .set_author(author.name(format!("{} {} Banned", user.tag(), self.get_name())).clone())
            .field("User", user.mention(), false)
            .color(0xFD4659)
            .fields(fields.clone());

        let mut dm_embed = CreateEmbed::default();
        match self {
            Self::Server => dm_embed.field("Appeal Forum", format!("[Click to appeal]({})", CONFIG.appeal_forum), false),
            Self::Scrim => dm_embed.field("Appeal Channel", CONFIG.appeal_channel.mention(), false),
        };

        dm_embed
            .title(format!("You were {} Banned", self.get_name()))
            .color(0xFD4659)
            .fields(fields)
            .footer(|f| {
                CONFIG.guild.to_guild_cached(ctx).unwrap().icon_url().map(|url| f.icon_url(url));
                f.text(CONFIG.guild.name(ctx).unwrap())
            });

        match self {
            Self::Server => {
                let is_banned = crate::consts::DATABASE.fetch_unbans().iter().any(|x| x.id == user_id.0);
                if is_banned {
                    embed.set_author(author.name(format!("{} {} Ban Updated", user.tag(), self.get_name())).clone()).color(0x0E87CC);
                    dm_embed.title(format!("Your {} Ban was Updated", self.get_name())).color(0x0E87CC);
                    if let Some(unban_date) = unban_date {
                        crate::consts::DATABASE.modify_unban_date(
                            *user_id.as_u64(),
                            unban_date
                        )?;
                    }else {
                        // Remove the scheduled unban to enforce the permanent duration
                        crate::consts::DATABASE.remove_entry("ScheduledUnbans", *user_id.as_u64())?;
                    }
                } else if let Some(unban_date) = unban_date {
                    crate::consts::DATABASE.add_unban(*user_id.as_u64(), unban_date)?;
                }

                let result = CONFIG.guild.ban_with_reason(&ctx, user_id, dmd, reason).await;
                if let Err(error) = result {
                    crate::consts::DATABASE.remove_entry("ScheduledUnbans", *user_id.as_u64())?;
                    return Err(Box::new(error));
                }
            }

            Self::Scrim => {
                let mut removed_roles = Vec::new();
                if let Some(ref member) = member {
                    let roles = member.roles(ctx).unwrap_or_default();
                    let mut new_roles = roles.iter().filter(|r| r.managed).map(|r| r.id).collect::<Vec<_>>();
                    new_roles.push(crate::CONFIG.banned);
                    removed_roles = roles.iter().map(|r| r.id).filter(|r| !new_roles.contains(r)).collect::<Vec<_>>();
                    member.edit(&ctx, |m| m.roles(new_roles)).await?;
                }

                let bans = crate::consts::DATABASE.fetch_scrim_unbans();
                let ban = bans.iter().find(|x| x.id == user_id.0);
                if let Some(ban) = ban {
                    embed.set_author(author.name(format!("{} {} Ban Updated", user.tag(), self.get_name())).clone()).color(0x0E87CC);
                    dm_embed.title(format!("Your {} Ban was Updated", self.get_name())).color(0x0E87CC);
                    let all_removed = [ban.roles.0.clone(), Ids::from(removed_roles).0]
                        .concat()
                        .into_iter().collect::<HashSet<_>>()
                        .into_iter().collect::<Vec<_>>();
                    crate::consts::DATABASE.modify_scrim_unban_date(
                        *user_id.as_u64(),
                        unban_date.unwrap(),
                        &Ids(all_removed)
                    )?;
                } else {
                    crate::consts::DATABASE.add_scrim_unban(
                        *user_id.as_u64(),
                        // NOTE: In the case of a `ScrimBan`, this is always `Some`
                        unban_date.unwrap(),
                        &removed_roles.into(),
                    )?;
                }
            }
        }

        send_ban_log(ctx, embed.clone()).await;

        if let Some(ref member) = member {
            let _ = member.user.dm(&ctx, |msg| msg.set_embed(dm_embed)).await;
        }

        Ok(embed)
    }

    fn get_unban_date<'a>(&self, duration_option: Option<String>) -> err_resp::Result<'a, Option<OffsetDateTime>> {
        let seconds = match duration_option {
            Some(resolvable) => parse_duration(resolvable)?.0,
            None => match self {
                Self::Scrim => 30*24*60*60,
                Self::Server => 0 // 0 indicates permanent
            }
        };
        if seconds > 0 {
            let now = OffsetDateTime::now_utc();
            let duration = Duration::from_secs(seconds);
            return Ok(Some(now + duration));
        }
        Ok(None) // None means permanent
    }

    pub async fn exec(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult
    {
        let executor = command.clone().member.unwrap();

        let user_id = UserId(command.get_str("user").unwrap().parse().unwrap());
        let user = user_id.to_user(&ctx).await
            .map_err(|_| ErrorResponse::with_title("Invalid User", "Please validate your user option and try again."))?;
        
        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason specified"));

        let dmd = command.get_bool("dmd").map(|_| 7).unwrap_or(0);

        let duration = command.get_str("duration");
        let unban_date = self.get_unban_date(duration)?;

        let member = CONFIG.guild.member(&ctx, user_id).await.ok();
        if let Some(ref member) = member {
            let roles = member.roles(ctx).unwrap_or_default();
            let cmd_roles = executor.roles(ctx).unwrap_or_default();
    
            let top_role = roles.iter().max();
            let cmd_top_role = cmd_roles.iter().max();

            if top_role >= cmd_top_role || member.user.bot {
                return Err(
                    ErrorResponse::with_title(
                        "Insufficient Permissions", 
                        format!("You do not have permission to ban {}!", member.mention())
                    )
                )?;
            }
        }
        
        if crate::consts::DATABASE.fetch_freezes_for(user_id.0).is_some() {
            super::unfreeze::unfreeze_user(ctx, user_id).await
                .map_err(|err| {
                    tracing::error!("Unfreeze Failed on {}: {}", user_id, err);
                    ErrorResponse::with_title(
                        "User is Frozen", 
                        format!("Unable to unfreeze {}!", user_id.mention())
                    )
                })?;
            
            let _ = command
                .create_followup_message(&ctx, |msg| {
                    msg.content(format!("Unfreezing {} before banning them...", user_id.mention()))
                        .flags(MessageFlags::EPHEMERAL)
                }).await;
        }

        let embed = self.ban(ctx, user, member, executor.user.id, unban_date, dmd, reason).await
            .map_err(|err| {
                tracing::error!("Ban Failed on {}: {}", user_id, err);
                ErrorResponse::with_title(
                    "Ban Failed", 
                    format!("Unable to ban {} at the moment!", user_id.mention())
                )
            })?;

        let mut resp = CreateInteractionResponseData::default();
        resp.add_embed(embed);
        Ok(Some(resp))
    }
}

pub struct Ban;

#[async_trait]
impl InteractionHandler for Ban {

    fn name(&self) -> String {
        String::from("ban")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec!(crate::CONFIG.support, crate::CONFIG.trial_support))
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG.guild
            .create_application_command(&ctx, |c| {
                c
                    .name(self.name())
                    .description("Bans a user from the server. (Do not confuse with /scrimban for scrim bans)")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to ban")
                            .required(true)
                            .kind(command::CommandOptionType::User)
                    })
                    .create_option(|o| {
                        o.name("reason")
                            .description("Reason for the ban")
                            .required(true)
                            .kind(command::CommandOptionType::String)
                    })
                    .create_option(|o| {
                        o.name("duration")
                            .description("The ban duration (e.g. 10s 15m 20h 16w 20months 1y). [Default: forever]")
                            .required(false)
                            .kind(command::CommandOptionType::String)
                    })
                    .create_option(|o| {
                        o.name("delete_messages")
                            .description("Should the last 7d of messages be removed?")
                            .required(false)
                            .kind(command::CommandOptionType::Boolean)
                    })
            })
            .await?;
        Ok(())
    }

    fn initial_response(&self, _interaction_type: interaction::InteractionType) -> InitialInteractionResponse {
        InitialInteractionResponse::DeferEphemeralReply
    }

    async fn handle_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult {
        BanType::Server.exec(ctx, command).await
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

pub struct ScrimBan;

#[async_trait]
impl InteractionHandler for ScrimBan {

    fn name(&self) -> String {
        String::from("scrimban")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec!(crate::CONFIG.ss_support, crate::CONFIG.support, crate::CONFIG.trial_support))
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Bans a user from playing scrims. (Do not confuse with /ban for server bans)")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to ban")
                            .required(true)
                            .kind(command::CommandOptionType::User)
                    })
                    .create_option(|o| {
                        o.name("reason")
                            .description("Reason for the ban")
                            .required(true)
                            .kind(command::CommandOptionType::String)
                    })
                    .create_option(|o| {
                        o.name("duration")
                            .description("The ban duration (e.g. 10s 15m 20h 16w 20months 1y). [Default: 30d]")
                            .required(false)
                            .kind(command::CommandOptionType::String)
                    })
            })
            .await?;
        Ok(())
    }
    
    fn initial_response(&self, _interaction_type: interaction::InteractionType) -> InitialInteractionResponse {
        InitialInteractionResponse::DeferEphemeralReply
    }
    
    async fn handle_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult {
        BanType::Scrim.exec(ctx, command).await
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

// TODO: list all (scrim)bans
// pub struct Bans;
// pub struct ScrimBans;
