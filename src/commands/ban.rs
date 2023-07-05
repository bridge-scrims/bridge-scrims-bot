use std::{collections::HashSet, time::Duration};
use time::OffsetDateTime;

use serenity::{
    async_trait,
    builder::{CreateEmbed, CreateEmbedAuthor, CreateInteractionResponseData},
    client::Context,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction, MessageFlags,
    },
    model::prelude::*,
};

use crate::{consts::CONFIG, db::Ids};
use bridge_scrims::{interaction::*, parse_durations::Duration as ParsedDuration};

pub struct ScrimBan;

#[async_trait]
impl InteractionHandler for ScrimBan {
    fn name(&self) -> String {
        String::from("scrimban")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec![
            crate::CONFIG.ss_support,
            crate::CONFIG.support,
            crate::CONFIG.trial_support,
        ])
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

    fn initial_response(
        &self,
        _interaction_type: interaction::InteractionType,
    ) -> InitialInteractionResponse {
        InitialInteractionResponse::DeferEphemeralReply
    }

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        let executor = command.member.as_ref().unwrap();
        let to_ban = UserId(command.get_str("user").unwrap().parse().unwrap());
        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason specified"));

        let duration = command.get_str("duration");
        let unban_date = get_unban_date(duration)?;

        let member = CONFIG.guild.member(&ctx, to_ban).await.ok();
        if let Some(ref member) = member {
            let roles = member.roles(ctx).unwrap_or_default();
            let cmd_roles = executor.roles(ctx).unwrap_or_default();

            let top_role = roles.iter().max();
            let cmd_top_role = cmd_roles.iter().max();

            if top_role >= cmd_top_role || member.user.bot {
                return Err(ErrorResponse::with_title(
                    "Insufficient Permissions",
                    format!("You do not have permission to ban {}!", member.mention()),
                ))?;
            }
        }

        if crate::consts::DATABASE
            .fetch_freezes_for(to_ban.0)
            .is_some()
        {
            super::unfreeze::unfreeze_user(ctx, to_ban)
                .await
                .map_err(|err| {
                    tracing::error!("Unfreeze Failed on {}: {}", to_ban, err);
                    ErrorResponse::with_title(
                        "User is Frozen",
                        format!("Unable to unfreeze {}!", to_ban.mention()),
                    )
                })?;

            let _ = command
                .create_followup_message(&ctx, |msg| {
                    msg.content(format!(
                        "Unfreezing {} before banning them...",
                        to_ban.mention()
                    ))
                    .flags(MessageFlags::EPHEMERAL)
                })
                .await;
        }

        let embed = scrim_ban(ctx, to_ban, executor.user.id, unban_date, reason).await?;
        let mut resp = CreateInteractionResponseData::default();
        resp.add_embed(embed);
        Ok(Some(resp))
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

fn parse_duration<'a>(resolvable: String) -> err_resp::Result<'a, ParsedDuration> {
    let duration = resolvable.parse::<ParsedDuration>();
    if duration.is_err() {
        return Err(ErrorResponse::with_title(
            "Invalid Duration",
            "Please provide a valid ban duration (e.g. 5m 30d).",
        ))?;
    }
    Ok(duration.unwrap())
}

fn get_unban_date<'a>(duration_option: Option<String>) -> err_resp::Result<'a, OffsetDateTime> {
    let seconds = match duration_option {
        Some(resolvable) => parse_duration(resolvable)?.0,
        None => 30 * 24 * 60 * 60,
    };
    let now = OffsetDateTime::now_utc();
    let duration = Duration::from_secs(seconds);
    Ok(now + duration)
}

pub async fn scrim_ban(
    ctx: &Context,
    to_ban_id: UserId,
    executor_id: UserId,
    unban_date: OffsetDateTime,
    reason: String,
) -> crate::Result<CreateEmbed> {
    let to_ban = to_ban_id.to_user(ctx).await?;
    let bans = crate::consts::DATABASE.fetch_scrim_unbans();
    let existing = bans.iter().find(|x| x.id == to_ban.id.0);

    let mut author = CreateEmbedAuthor::default();
    author.icon_url(
        to_ban
            .avatar_url()
            .unwrap_or_else(|| to_ban.default_avatar_url()),
    );

    let mut fields = Vec::new();
    fields.push((
        "Expires",
        format!(
            "{}<t:{}:R>",
            existing.map_or(String::default(), |e| format!(
                "{} **➔** ",
                e.date.map_or(String::from("*Expired*"), |d| format!(
                    "<t:{}:R>",
                    d.unix_timestamp()
                ))
            )),
            unban_date.unix_timestamp()
        ),
        true,
    ));

    fields.push(("Staff", executor_id.mention().to_string(), true));
    fields.push(("Reason", format!("```{}```", reason), false));

    let mut embed = CreateEmbed::default();
    embed
        .set_author(
            author
                .name(format!("{} Scrim Banned", to_ban.tag()))
                .clone(),
        )
        .field("User", to_ban.mention(), true)
        .color(0xFD4659)
        .fields(fields.clone());

    let mut dm_embed = CreateEmbed::default();
    dm_embed
        .title("You were banned from queuing Scrims")
        .field("Appeal Channel", CONFIG.appeal_channel.mention(), true)
        .color(0xFD4659)
        .fields(fields)
        .footer(|f| {
            CONFIG
                .guild
                .to_guild_cached(ctx)
                .unwrap()
                .icon_url()
                .map(|url| f.icon_url(url));
            f.text(CONFIG.guild.name(ctx).unwrap())
        });

    let mut removed_roles = Vec::new();
    let member = CONFIG.guild.member(ctx, to_ban.id).await;
    if let Ok(member) = member {
        let roles = member.roles(ctx).unwrap_or_default();
        let mut new_roles = roles
            .iter()
            .filter(|r| r.managed)
            .map(|r| r.id)
            .collect::<Vec<_>>();
        new_roles.push(crate::CONFIG.banned);
        removed_roles = roles
            .iter()
            .map(|r| r.id)
            .filter(|r| !new_roles.contains(r))
            .collect::<Vec<_>>();

        member.edit(&ctx, |m| m.roles(new_roles)).await?;
    }

    if let Some(ban) = existing {
        embed
            .set_author(
                author
                    .name(format!("{} Scrim Ban Updated", to_ban.tag()))
                    .clone(),
            )
            .color(0x0E87CC);
        dm_embed.title("Your Scrim Ban was Updated").color(0x0E87CC);

        let all_removed = [ban.roles.0.clone(), Ids::from(removed_roles).0]
            .concat()
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        crate::consts::DATABASE.modify_scrim_unban(
            to_ban.id.0,
            Some(unban_date),
            &Ids(all_removed),
        )?;
    } else {
        crate::consts::DATABASE.add_scrim_unban(
            to_ban.id.0,
            Some(unban_date),
            &removed_roles.into(),
        )?;
    }

    let _ = CONFIG
        .support_bans
        .send_message(&ctx, |msg| msg.set_embed(embed.clone()))
        .await
        .map_err(|err| tracing::error!("Failed to log to #bans: {}", err));

    let _ = to_ban.dm(ctx, |msg| msg.set_embed(dm_embed)).await;
    Ok(embed)
}
