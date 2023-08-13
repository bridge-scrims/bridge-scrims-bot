use std::time::Duration;
use std::{collections::HashMap, collections::HashSet, sync::Arc};

use lazy_static::lazy_static;

use serenity::async_trait;
use serenity::builder::CreateEmbed;
use serenity::client::{Context, EventHandler};
use serenity::model::application::interaction::Interaction;
use serenity::model::channel::{Message, MessageType, ReactionType};
use serenity::model::gateway::Ready;
use serenity::model::prelude::*;
use tokio::sync::Mutex;

use bridge_scrims::interaction::handler::InteractionHandler;

use crate::commands;
use crate::commands::screenshare::unban::scrim_unban;
use crate::consts::CONFIG;
use crate::consts::DATABASE as database;
use crate::db::CustomReaction;
use crate::features::expanding_channels::ExpandingChannels;

lazy_static! {
    pub static ref HANDLERS: Vec<Box<dyn InteractionHandler>> = vec![
        commands::council::Council::new(),
        commands::notes::Notes::new(),
        commands::prefabs::Prefab::new(),
        commands::teams::TeamsCommand::new(),
        commands::captains::CaptainsCommand::new(),
        commands::purge::Purge::new(),
        commands::reaction::Reaction::new(),
        commands::reaction::DelReaction::new(),
        commands::reaction::ListReactions::new(),
        commands::reload::Reload::new(),
        commands::ping::Ping::new(),
        commands::screenshare::ban::ScrimBan::new(),
        commands::screenshare::unban::ScrimUnban::new(),
        commands::screenshare::screenshare::Screenshare::new(),
        commands::screenshare::close::Close::new(),
        commands::screenshare::freeze::Freeze::new(),
        commands::screenshare::unfreeze::Unfreeze::new(),
        commands::screenshare::ticket::Ticket::new(),
        commands::screenshare::list_bans::ListBans::new(),
        commands::screenshare::screensharers::Screensharers::new(),
    ];
    pub static ref REACTIONS: Arc<Mutex<HashMap<String, CustomReaction>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub struct Handler {
    init: Mutex<bool>,
}

impl Handler {
    pub fn new() -> Handler {
        Handler {
            init: Mutex::new(false),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, data: Ready) {
        tracing::info!("Connected to discord as {}", data.user.tag());
    }

    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let mut init = self.init.lock().await;
        if !*init {
            *init = true;

            tokio::spawn(register_commands(ctx.clone()));
            tokio::spawn(update_reactions_loop());
            ExpandingChannels::init(&ctx);

            for handler in &*HANDLERS {
                handler.init(&ctx).await;
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        if new.channel_id.is_none() || new.guild_id == Some(CONFIG.guild) {
            ExpandingChannels::on_voice_update(&ctx, old.as_ref(), &new).await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Autocomplete(interaction) = &interaction {
            if let Some(handler) = HANDLERS
                .iter()
                .find(|x| x.is_handler(interaction.data.name.clone()))
            {
                if let Err(err) = handler.on_autocomplete(&ctx, interaction).await {
                    tracing::error!("{} autocomplete failed: {}", handler.name(), err);
                }
            }
        }

        if let Interaction::ApplicationCommand(interaction) = &interaction {
            if let Some(handler) = HANDLERS
                .iter()
                .find(|x| x.is_handler(interaction.data.name.clone()))
            {
                if let Err(err) = handler.on_command(&ctx, interaction).await {
                    tracing::error!("{} command failed: {}", handler.name(), err);
                }
            }
        }

        if let Interaction::MessageComponent(interaction) = &interaction {
            let mut args = interaction.data.custom_id.split(':').collect::<Vec<_>>();
            let name = args.drain(..1).next();
            if let Some(name) = name {
                if let Some(handler) = HANDLERS.iter().find(|x| x.is_handler(name.to_string())) {
                    if let Err(err) = handler.on_component(&ctx, interaction, &args).await {
                        tracing::error!("{} component failed: {}", handler.name(), err);
                    }
                }
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot || msg.guild_id != Some(CONFIG.guild) {
            return;
        }

        if msg
            .content
            .to_ascii_lowercase()
            .replace(' ', "")
            .contains("shmill")
        {
            if let Err(err) = msg
                .react(
                    &ctx,
                    CONFIG
                        .guild
                        .to_guild_cached(&ctx)
                        .unwrap()
                        .emojis
                        .get(&CONFIG.shmill_emoji)
                        .unwrap()
                        .clone(),
                )
                .await
            {
                tracing::error!("{}", err);
            }
        }

        if msg.content.to_ascii_lowercase() == "ratio"
            || msg.content.to_ascii_lowercase().replace(' ', "") == "counterratio"
        {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👍".into())).await {
                tracing::error!("{}", err);
            }
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👎".into())).await {
                tracing::error!("{}", err);
            }
        }

        if msg.channel_id == CONFIG.q_and_a_channel {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👍".into())).await {
                tracing::error!("{}", err);
            }
            match msg.member(&ctx).await {
                Ok(m) => {
                    let mut member = m.clone();
                    if let Err(err) = member.add_role(&ctx.http, CONFIG.q_and_a_role).await {
                        tracing::error!("{}", err)
                    }
                }
                Err(err) => tracing::error!("{}", err),
            }
        }

        if msg.channel_id.as_u64() == CONFIG.clips.as_u64() {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👍".into())).await {
                tracing::error!("{}", err);
            }
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👎".into())).await {
                tracing::error!("{}", err);
            }

            if let Err(err) = msg
                .channel_id
                .create_public_thread(&ctx, msg.id, |thread| {
                    thread.name(format!("Clip by {}!", msg.author.name))
                })
                .await
            {
                tracing::error!("{}", err);
            }
        }

        if CONFIG.upvote_downvote_channels.contains(&msg.channel_id) {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("✅".into())).await {
                tracing::error!("{}", err);
            }
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("❌".into())).await {
                tracing::error!("{}", err);
            }
        }

        if CONFIG.like_react_channels.contains(&msg.channel_id) {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👍".into())).await {
                tracing::error!("{}", err);
            }
        }

        match msg.kind {
            MessageType::MemberJoin => {
                if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👋".into())).await {
                    tracing::error!("{}", err);
                }
            }
            MessageType::NitroBoost
            | MessageType::NitroTier1
            | MessageType::NitroTier2
            | MessageType::NitroTier3 => {
                if let Err(err) = msg.react(&ctx, ReactionType::Unicode("🎉".into())).await {
                    tracing::error!("{}", err);
                }
                if let Err(err) = CONFIG
                    .booster_info
                    .send_message(&ctx.http, |m| {
                        m.add_embed(|em| {
                            em.title(format!("{} has boosted the server!", msg.author.tag()))
                                .description(format!(
                                    "Thank you for boosting the server <@!{}>",
                                    msg.author.id
                                ))
                                .thumbnail(msg.author.avatar_url().unwrap_or_default())
                        })
                        .reactions([ReactionType::Unicode("🎉".into())])
                    })
                    .await
                {
                    tracing::error!("{}", err)
                }
            }
            _ => {}
        }

        let reactions = REACTIONS.lock().await;
        if let Some(reaction) = reactions.get(&msg.content.to_ascii_lowercase()) {
            if let Err(err) = msg
                .react(
                    &ctx,
                    ReactionType::try_from(reaction.emoji.clone()).unwrap(),
                )
                .await
            {
                if format!("{}", err)
                    .to_ascii_lowercase()
                    .contains("unknown emoji")
                    || format!("{}", err)
                        .to_ascii_lowercase()
                        .contains("invalid form body")
                {
                    if let Err(err) = database.remove_custom_reaction(reaction.user_id).await {
                        tracing::error!("Error in removal of reaction: {}", err);
                    }
                    if let Err(err) = msg.reply(&ctx, format!(
                        "Hey <@{}>, it looks like the custom reaction which you added has an invalid emoji. It's been removed from the database, make sure that anything which you add is a default emoji.",
                        &reaction.user_id),
                    )
                        .await
                    {
                        tracing::error!("{}", err);
                    }
                } else {
                    tracing::error!("Error in addition of reaction: {}", err);
                }
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        if let Err(err) = CONFIG.member_count.update(&ctx, member.guild_id).await {
            tracing::error!("Error when updating member count: {}", err)
        }
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        _user: User,
        _optional_member: Option<Member>,
    ) {
        if let Err(err) = CONFIG.member_count.update(&ctx, guild_id).await {
            tracing::error!("Error when updating member count: {}", err)
        }
    }

    async fn guild_member_update(&self, ctx: Context, _old_member: Option<Member>, member: Member) {
        if member.guild_id == CONFIG.guild {
            let _ = check_booster(&ctx, &member)
                .await
                .map_err(|err| tracing::error!("Error while checking for booster: {}", err));

            let _ = check_scrim_banned(&ctx, &member)
                .await
                .map_err(|err| tracing::error!("Error while checking for scrim banned: {}", err));
        }
    }

    async fn channel_create(&self, ctx: Context, channel: &GuildChannel) {
        if channel.guild_id == CONFIG.guild {
            check_channel_permissions(&ctx, channel.id, &channel.permission_overwrites).await;
        }
    }

    async fn category_create(&self, ctx: Context, category: &ChannelCategory) {
        if category.guild_id == CONFIG.guild {
            check_channel_permissions(&ctx, category.id, &category.permission_overwrites).await;
        }
    }
}

lazy_static! {
    pub static ref MUTED_OVERWRITE_TYPE: PermissionOverwriteType =
        PermissionOverwriteType::Role(CONFIG.muted_role);
    pub static ref MUTED_DENY_PERMISSIONS: Permissions = Permissions::SEND_MESSAGES
        | Permissions::SEND_MESSAGES_IN_THREADS
        | Permissions::CREATE_PUBLIC_THREADS
        | Permissions::CREATE_PRIVATE_THREADS
        | Permissions::ADD_REACTIONS
        | Permissions::SPEAK;
}

async fn check_channel_permissions(
    ctx: &Context,
    channel: ChannelId,
    permissions: &[PermissionOverwrite],
) {
    let existing = permissions.iter().find(|p| p.kind == *MUTED_OVERWRITE_TYPE);
    if !existing.map_or(false, |o| o.deny.contains(*MUTED_DENY_PERMISSIONS)) {
        let mut allow = existing.map_or(Permissions::empty(), |o| o.allow);
        let mut deny = existing.map_or(Permissions::empty(), |o| o.deny);

        allow.remove(*MUTED_DENY_PERMISSIONS);
        deny.insert(*MUTED_DENY_PERMISSIONS);

        let _ = channel
            .create_permission(
                ctx,
                &PermissionOverwrite {
                    allow,
                    deny,
                    kind: *MUTED_OVERWRITE_TYPE,
                },
            )
            .await
            .map_err(|err| {
                tracing::error!("Error while fixing channel's muted permissions: {}", err)
            });
    }
}

async fn check_booster(ctx: &Context, member: &Member) -> crate::Result<()> {
    let booster = member.premium_since.is_some()
        || member.permissions(ctx).map_or(false, |p| p.administrator());

    if booster {
        return Ok(());
    }

    let custom_reactions = database
        .fetch_custom_reactions_for(member.user.id.0)
        .await?;
    if !custom_reactions.is_empty() {
        // if the user's server boost runs out
        let mut embed = CreateEmbed::default();
        embed.title(format!("{}'s reaction has been removed", member.user.tag()));
        embed.description("No longer has booster and is not an administrator.");

        database.remove_custom_reaction(member.user.id.0).await?;
        CONFIG
            .reaction_logs
            .send_message(&ctx, |msg| msg.set_embed(embed.clone()))
            .await?;

        update_reactions_map().await;
    }

    if let Some(roles) = member.roles(ctx) {
        if roles.iter().any(|r| CONFIG.color_roles.contains(&r.id)) {
            let roles_without_colors = roles
                .iter()
                .filter(|r| !CONFIG.color_roles.contains(&r.id))
                .map(|r| r.id)
                .collect::<Vec<_>>();

            member.edit(&ctx, |m| m.roles(roles_without_colors)).await?;
        }
    }

    Ok(())
}

async fn check_scrim_banned(ctx: &Context, member: &Member) -> crate::Result<()> {
    let unbans = crate::consts::DATABASE.fetch_scrim_unbans().await?;
    let unban = unbans.iter().find(|x| x.user_id == member.user.id.0);
    if let Some(unban) = unban {
        if unban.is_expired() {
            scrim_unban(ctx, None, unban, String::from("Ban Expired")).await?;
        } else {
            let roles = member.roles(ctx).unwrap_or_default();
            if !roles
                .iter()
                .all(|r| r.managed || r.id == crate::CONFIG.banned)
                || !roles.iter().any(|r| r.id == crate::CONFIG.banned)
            {
                let mut new_roles = roles
                    .iter()
                    .filter(|r| r.managed)
                    .map(|r| r.id)
                    .collect::<Vec<_>>();
                new_roles.push(crate::CONFIG.banned);

                let removed_roles = roles
                    .iter()
                    .map(|r| r.id)
                    .filter(|r| !new_roles.contains(r))
                    .map(|r| r.0)
                    .collect::<Vec<_>>();

                member.edit(&ctx, |m| m.roles(new_roles)).await?;

                if !removed_roles.iter().all(|r| unban.roles.contains(r)) {
                    let all_removed = [unban.roles.clone(), removed_roles]
                        .concat()
                        .into_iter()
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>();

                    crate::consts::DATABASE
                        .modify_scrim_unban(
                            *member.user.id.as_u64(),
                            unban.expires_at,
                            &all_removed,
                        )
                        .await?;
                }
            }
        }
    }

    Ok(())
}

async fn update_reactions_loop() {
    loop {
        update_reactions_map().await;
        tokio::time::sleep(Duration::from_secs(60 * 60 * 2)).await;
    }
}

pub async fn update_reactions_map() {
    let mut lock = REACTIONS.lock().await;
    let mut x = HashMap::new();
    for reaction in database
        .fetch_custom_reactions()
        .await
        .expect("Failed to fetch reactions from the database")
    {
        x.insert(reaction.trigger.to_ascii_lowercase(), reaction);
    }
    *lock = x;
}

pub async fn register_commands(ctx: Context) -> Result<(), String> {
    #[allow(unused_mut)]
    let mut res = Ok(());
    let guild_commands = CONFIG.guild.get_application_commands(&ctx.http).await;

    for handler in &*HANDLERS {
        let name = handler.name();
        // ignore any commands that we have already registered.
        if let Ok(ref cmds) = guild_commands {
            if cmds.iter().any(|cmd| handler.is_handler(cmd.name.clone())) {
                continue;
            }
        }
        let result = handler.register(&ctx).await.map_err(|x| x.to_string());
        if let Err(ref err) = result.as_ref() {
            tracing::error!("Could not register command {}: {}", name, err);
        }
        res = res.and(result);
    }
    res
}
