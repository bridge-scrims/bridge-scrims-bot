use std::{collections::HashSet, time::Duration};

use serenity::{
    async_trait,
    builder::{CreateEmbed, CreateInteractionResponseData},
    client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::prelude::*,
};

use crate::{consts::CONFIG, consts::DATABASE, db};
use bridge_scrims::interaction::*;

pub struct ScrimUnban;

#[async_trait]
impl InteractionHandler for ScrimUnban {
    fn name(&self) -> String {
        String::from("scrimunban")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec![
            crate::CONFIG.ss_support,
            crate::CONFIG.support,
            crate::CONFIG.trial_support,
        ])
    }

    async fn init(&self, ctx: &Context) {
        tokio::spawn(scrim_unban_update_loop(ctx.clone()));
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Unbans a user from queuing in scrims")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to unban")
                            .required(true)
                            .kind(command::CommandOptionType::User)
                    })
                    .create_option(|o| {
                        o.name("reason")
                            .description("Reason this user is being unbanned")
                            .kind(command::CommandOptionType::String)
                            .required(false)
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
        let user = command.get_str("user").unwrap();
        let user_id = UserId(user.parse().unwrap_or_default());
        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason provided"));

        let unban = DATABASE
            .fetch_scrim_unbans()
            .await?
            .into_iter()
            .find(|x| x.user_id == user_id.0)
            .ok_or_else(|| ErrorResponse::message(format!("{} is not banned.", user)))?;

        let embed = scrim_unban(ctx, Some(command.user.id), &unban, reason).await?;

        let mut resp = CreateInteractionResponseData::default();
        resp.add_embed(embed);
        Ok(Some(resp))
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

pub async fn scrim_unban(
    ctx: &Context,
    staff_id: Option<UserId>, // If staff id is not provided, it is assumed that the ban expired
    unban: &db::ScrimUnban,
    reason: String,
) -> crate::Result<CreateEmbed> {
    let to_unban = UserId(unban.user_id).to_user(ctx).await?;

    let mut fields = Vec::new();

    if let Some(staff_id) = staff_id {
        fields.push(("Staff", staff_id.mention().to_string(), true));
    }
    fields.push(("Reason", format!("```{}```", reason), false));

    let mut embed = CreateEmbed::default();
    embed
        .author(|a| {
            a.name(format!("{} Unbanned", to_unban.tag())).icon_url(
                to_unban
                    .avatar_url()
                    .unwrap_or_else(|| to_unban.default_avatar_url()),
            )
        })
        .field("User", to_unban.mention(), true)
        .color(0x20BF72)
        .fields(fields.clone());

    let mut dm_embed = CreateEmbed::default();
    dm_embed
        .title("You were Unbanned")
        .color(0x20BF72)
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

    let log_unban = async {
        if !unban.was_logged() {
            let logged = CONFIG
                .support_bans
                .send_message(ctx, |msg| msg.set_embed(embed.clone()))
                .await;

            let _ = to_unban.dm(ctx, |msg| msg.set_embed(dm_embed)).await;

            if let Err(err) = logged {
                tracing::error!("Failed to log unban to #bans: {}", err);
                return Err(err);
            }
        }
        Ok(())
    };

    let member = CONFIG.guild.member(ctx, to_unban.id).await;
    if let Ok(member) = member {
        sqlx::query!(
            "DELETE FROM scheduled_scrim_unban WHERE user_id = $1",
            unban.user_id as i64
        )
        .execute(&DATABASE.get())
        .await?;

        let taken_roles: Vec<RoleId> = unban.roles.iter().map(|r| RoleId(*r)).collect();
        let new_roles = taken_roles
            .into_iter()
            .chain([CONFIG.member_role].into_iter())
            .chain(member.roles(ctx).unwrap().iter().map(|r| r.id))
            .filter(|r| {
                r.0 != CONFIG.banned.0
                    && ctx
                        .cache
                        .guild_roles(CONFIG.guild.0)
                        .unwrap()
                        .contains_key(r)
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let res = member.edit(ctx, |m| m.roles(new_roles)).await;
        if res.is_err() {
            let _ = DATABASE
                .add_scrim_unban(unban.user_id, unban.expires_at, &unban.roles)
                .await
                .map_err(|err| {
                    tracing::error!(
                        "Failed to re-add scrim unban after giving back roles back failed: {}",
                        err
                    )
                });
            res?;
        }
        let _ = log_unban.await;
    } else {
        if unban.was_logged() {
            let mut embed = CreateEmbed::default();
            embed
                .color(0x20BF72)
                .description(format!("**{}** was already unbanned.", to_unban.tag()));
            return Ok(embed);
        }

        log_unban.await?;
        let _  = sqlx::query!(
            "UPDATE scheduled_scrim_unban SET expires_at = NULL WHERE user_id = $1",
            unban.user_id as i64
        )
        .execute(&DATABASE.get())
        .await;
    }

    Ok(embed)
}

async fn scrim_unban_update_loop(ctx: Context) {
    let database = &crate::consts::DATABASE;
    loop {
        if let Ok(unbans) = database.fetch_scrim_unbans().await {
            for unban in unbans {
                if unban.is_expired() {
                    let target = unban.user_id;
                    let res = scrim_unban(&ctx, None, &unban, String::from("Ban Expired")).await;
                    if let Err(err) = res {
                        tracing::error!(
                            "Failed to unban {} from scrims upon expiration: {}",
                            target,
                            err
                        );
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(3 * 60)).await;
    }
}
