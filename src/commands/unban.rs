use time::OffsetDateTime;
use std::time::Duration;

use serenity::{
    async_trait,
    client::Context,
    builder::{CreateEmbed, CreateInteractionResponseData, CreateEmbedAuthor},

    model::prelude::*,
    model::application::interaction::application_command::ApplicationCommandInteraction
};

use bridge_scrims::interaction::*;
use crate::consts::CONFIG;

#[derive(Debug)]
pub enum UnbanType {
    Scrim,
    Server,
}

pub enum UnbanEntry {
    Scrim(crate::model::ScrimUnban),
    Server(Ban),
}

async fn send_ban_log(ctx: &Context, embed: CreateEmbed) {
    let _ = CONFIG.support_bans
        .send_message(&ctx, |msg| msg.set_embed(embed.clone())).await
        .map_err(|e| tracing::error!("Failed to log to support_bans: {}", e));
}

impl UnbanType {

    fn get_comment(&self) -> String {
        match self {
            Self::Server => String::from(""),
            Self::Scrim => String::from("from playing scrims")
        }
    }

    pub async fn exec(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult
    {
        let user = command.get_str("user").unwrap();
        let user_id = UserId(user.parse().unwrap_or_default());
        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason provided"));
        
        let entry = match self {

            UnbanType::Scrim => crate::consts::DATABASE
                .fetch_scrim_unbans()
                .into_iter()
                .find(|x| x.id == user_id.0)
                .map(UnbanEntry::Scrim),

            UnbanType::Server => {
                let bans = CONFIG.guild.bans(&ctx).await?;
                bans.into_iter()
                    .find(|b| b.user.id == user_id || user == b.user.tag())
                    .map(UnbanEntry::Server)
            }

        }.ok_or_else(|| ErrorResponse::message(format!("{} is not banned.", user)))?;

        let embed = self
            .unban(ctx, Some(command.user.id), entry, reason)
            .await?;

        let mut resp = CreateInteractionResponseData::default();
        resp.add_embed(embed);
        Ok(Some(resp))
    }

    pub async fn unban(
        &self,
        ctx: &Context,
        staff_id: Option<UserId>, // If staff id is not provided, it is assumed that the ban expired
        unban_entry: UnbanEntry,
        reason: String
    ) -> crate::Result<CreateEmbed> 
    {
        let to_unban = match unban_entry {
            UnbanEntry::Scrim(entry) => CONFIG.guild.member(&ctx, entry.id).await
                .map(|m| m.user)
                .map_err(|_| 
                    ErrorResponse::with_title(
                        "No Member", 
                        "You can't unban someone from scrims who is not in the server because they wouldn't get their roles back!"
                    )
                ),
            UnbanEntry::Server(entry) => Ok(entry.user)
        }?;

        let mut fields = Vec::new();

        if let Some(staff_id) = staff_id {
            fields.push(("Staff", staff_id.mention().to_string(), false));
        }
        fields.push(("Reason", format!("`{}`", reason), false));

        let mut embed_author = CreateEmbedAuthor::default();
        embed_author.name(format!("{} Unbanned {}", to_unban.tag(), self.get_comment()));
        embed_author.icon_url(to_unban.avatar_url().unwrap_or_else(|| to_unban.default_avatar_url()));

        let mut embed = CreateEmbed::default();
        embed
            .set_author(embed_author)
            .field("User", to_unban.mention(), false)
            .color(0x20BF72)
            .fields(fields.clone());

        let mut dm_embed = CreateEmbed::default();
        dm_embed
            .title(format!("You were Unbanned {}", self.get_comment()))
            .color(0x20BF72)
            .fields(fields)
            .footer(|f| {
                CONFIG.guild.to_guild_cached(ctx).unwrap().icon_url().map(|url| f.icon_url(url));
                f.text(CONFIG.guild.name(ctx).unwrap())
            });

        match self {

            Self::Server => {
                CONFIG.guild.unban(&ctx, to_unban.id).await?;
                // Permanent server bans are not in the database, so this result may need to be an error
                let _ = crate::consts::DATABASE.remove_entry("ScheduledUnbans", to_unban.id.0);
            }

            Self::Scrim => {
                let member = CONFIG.guild.member(&ctx, to_unban.id).await?;
                let unban = crate::consts::DATABASE
                    .fetch_scrim_unbans()
                    .into_iter()
                    .find(|x| x.id == to_unban.id.0)
                    .unwrap();

                let mut roles: Vec<RoleId> = unban.roles.into();
                if !roles.contains(&CONFIG.member_role) {
                    roles.push(CONFIG.member_role)
                }

                let keep_roles = member.roles(ctx)
                    .unwrap_or_default().iter().filter(|r| r.managed).map(|r| r.id).collect::<Vec<_>>();

                let new_roles = keep_roles.iter()
                    .chain(
                        roles.iter()
                            .filter(|r| ctx.cache.guild_roles(CONFIG.guild.0).unwrap().contains_key(r))
                    );
                
                member.edit(&ctx, |m| m.roles(new_roles)).await?;

                // Member already has their roles back so it doesn't really matter if this is an error
                let _ = crate::consts::DATABASE.remove_entry("ScheduledScrimUnbans", to_unban.id.0);
            }
        }
        
        send_ban_log(ctx, embed.clone()).await; // log message
        let _ = to_unban.dm(&ctx, |msg| msg.set_embed(dm_embed)).await; // dm message
        Ok(embed) // command output
    }
}

pub struct Unban;

#[async_trait]
impl InteractionHandler for Unban {

    fn name(&self) -> String {
        String::from("unban")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec!(crate::CONFIG.support, crate::CONFIG.trial_support))
    }

    async fn init(&self, ctx: &Context) {
        tokio::spawn(unban_update_loop(ctx.clone()));
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG.guild
            .create_application_command(&ctx, |c| {
                c
                    .name(self.name())
                    .description("Unbans the user from the server. (Do not confuse with /scrimunban for scrim bans)")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|o| {
                        o.name("user")
                            .description("The user id or tag to unban")
                            .required(true)
                            .kind(command::CommandOptionType::String)
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

    fn initial_response(&self, _interaction_type: interaction::InteractionType) -> InitialInteractionResponse {
        InitialInteractionResponse::DeferEphemeralReply
    }

    async fn handle_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult {
        UnbanType::Server.exec(ctx, command).await
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

async fn unban_update_loop(ctx: Context) {
    let database = &crate::consts::DATABASE;

    loop {
        let unbans = database.fetch_unbans();
        let bans = CONFIG.guild.bans(&ctx).await.unwrap_or_default();
        let now = OffsetDateTime::now_utc();

        for unban in unbans {
            if unban.date <= now {
                if let Some(ban) = bans.iter().find(|b| b.user.id.0 == unban.id) {
                    let res = UnbanType::Server
                        .unban(
                            &ctx, None, UnbanEntry::Server(ban.clone()),
                            "Ban Expired".to_string(),
                        ).await;
                    if let Err(err) = res {
                        tracing::error!("Failed to unban {} upon expiration: {}", unban.id, err);
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
    }
}

pub struct ScrimUnban;

#[async_trait]
impl InteractionHandler for ScrimUnban {

    fn name(&self) -> String {
        String::from("scrimunban")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec!(crate::CONFIG.ss_support, crate::CONFIG.support, crate::CONFIG.trial_support))
    }

    async fn init(&self, ctx: &Context) {
        tokio::spawn(scrim_unban_update_loop(ctx.clone()));
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Unbans a user from scrims. (Do not confuse with /unban for server bans)")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|o| {
                        o.name("user")
                            .description("The user id or tag to unban")
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

    fn initial_response(&self, _interaction_type: interaction::InteractionType) -> InitialInteractionResponse {
        InitialInteractionResponse::DeferEphemeralReply
    }

    async fn handle_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult {
        UnbanType::Scrim.exec(ctx, command).await
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

async fn scrim_unban_update_loop(ctx: Context) {
    let database = &crate::consts::DATABASE;

    loop {
        for unban in database.fetch_scrim_unbans() {
            if unban.is_expired() {
                let member = CONFIG.guild.member(&ctx, unban.id).await;
                if let Ok(member) = member {
                    let res = UnbanType::Scrim
                        .unban(
                            &ctx,None, UnbanEntry::Scrim(unban),
                            "Ban Expired".to_string(),
                        ).await;
                    if let Err(err) = res {
                        tracing::error!("Failed to unban {} from scrims upon expiration: {}", member.user.id, err);
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(3 * 60)).await;
    }
}