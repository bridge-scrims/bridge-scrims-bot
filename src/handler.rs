use std::error::Error;
use std::time::Duration;
use std::{collections::HashMap, collections::HashSet, sync::Arc};

use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use regex::Regex;

use serenity::async_trait;
use serenity::builder::CreateEmbed;
use serenity::client::{Context, EventHandler};
use serenity::model::application::interaction::Interaction;
use serenity::model::channel::{Message, MessageType, ReactionType};
use serenity::model::gateway::Ready;
use serenity::model::prelude::*;
use serenity::utils::Color;
use tokio::sync::Mutex;

use bridge_scrims::interaction::handler::InteractionHandler;

use crate::commands;
use crate::consts::CONFIG;
use crate::consts::DATABASE as database;
use crate::db::{CustomReaction, Ids};

lazy_static! {
    pub static ref HANDLERS: Vec<Box<dyn InteractionHandler>> = vec![
        commands::council::Council::new(),
        commands::notes::Notes::new(),
        commands::prefabs::Prefab::new(),
        commands::timeout::Timeout::new(),
        commands::ban::Ban::new(),
        commands::unban::Unban::new(),
        commands::ban::ScrimBan::new(),
        commands::unban::ScrimUnban::new(),
        commands::roll::Roll::new(),
        commands::roll::Teams::new(),
        commands::purge::Purge::new(),
        commands::reaction::Reaction::new(),
        commands::reaction::DelReaction::new(),
        commands::reaction::ListReactions::new(),
        commands::screenshare::Screenshare::new(),
        commands::close::Close::new(),
        commands::freeze::Freeze::new(),
        commands::unfreeze::Unfreeze::new(),
        commands::ticket::Ticket::new(),
        commands::list_bans::ListBans::new(),
        commands::screensharers::Screensharers::new(),
        commands::reload::Reload::new(),
        commands::ping::Ping::new(),
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
    async fn ready(&self, ctx: Context, data: Ready) {
        tracing::info!("Connected to discord as {}", data.user.tag());
        let mut init = self.init.lock().await;
        if !*init {
            *init = true;
            for handler in &*HANDLERS {
                handler.init(&ctx).await;
            }
            tokio::spawn(update_reactions_loop());
        }
        // Errors are already handled
        let _ = register_commands(&ctx).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
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
        if msg.author.bot {
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
                        .emoji(&ctx, EmojiId(860966032952262716))
                        .await
                        .unwrap(),
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
        let member = msg.author.clone();

        let guild;
        if let Some(g) = msg.guild(&ctx) {
            guild = g;
        } else {
            tracing::warn!("Message from a user in a dm {:?}", msg);
            return;
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

        let roll_commands: Regex = Regex::new("^!(queue|roll|captains|teams|caps|team|captain|r|swag|townhalllevel10btw|anchans|scythepro|wael|api|gez|iamanchansbitch|wasim|unicorn|noodle|Limqo|!|h|eurth|QnVubnkgR2lybA|random)").unwrap();

        let channel = msg.channel_id.to_channel(&ctx.http).await.unwrap().guild();
        if roll_commands.is_match(&msg.content)
            && channel.as_ref().is_some()
            && channel.as_ref().unwrap().parent_id.is_some()
            && CONFIG
                .queue_categories
                .contains(&channel.unwrap().parent_id.unwrap())
        {
            let voice_state = guild.voice_states.get(&member.id);

            if voice_state.is_none() || voice_state.unwrap().channel_id.is_none() {
                let _ = msg
                    .reply(&ctx, "Please join a queue before using this command.")
                    .await;
                return;
            }

            let channel_id = voice_state.unwrap().channel_id.unwrap();
            let channel = channel_id
                .to_channel_cached(&ctx.cache)
                .unwrap()
                .guild()
                .unwrap();
            if !CONFIG
                .queue_categories
                .contains(&channel.parent_id.unwrap_or(ChannelId(0)))
            {
                let _ = msg
                    .reply(&ctx, "Please join a queue before using this command.")
                    .await;
                return;
            }

            let mut members = channel.members(&ctx.cache).await.unwrap();

            let user_limit: usize = channel.user_limit.unwrap_or(4).try_into().unwrap();

            if members.len() < user_limit {
                let _ = msg.reply(&ctx.http, "This queue is not full yet.").await;
                return;
            }

            members.shuffle(&mut rand::thread_rng());

            let _ = msg
                .channel_id
                .send_message(&ctx, |r| {
                    r.add_embed(|e| {
                        e.title("Team Captains:")
                            .field("First Captain", members[0].mention(), true)
                            .field("Second Captain", members[1].mention(), true)
                            .color(Color::new(0x1abc9c))
                            .footer(|f| f.text("Hint: use the /roll command to roll!"))
                    })
                    .reference_message(&msg)
                    .allowed_mentions(serenity::builder::CreateAllowedMentions::empty_parse)
                })
                .await;
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
        if CONFIG.upvote_downvote.contains(&msg.channel_id) {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("✅".into())).await {
                tracing::error!("{}", err);
            }
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("❌".into())).await {
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
                    if let Err(err) = database.remove_custom_reaction(reaction.user) {
                        tracing::error!("Error in removal of reaction: {}", err);
                    }
                    if let Err(err) = msg.reply(&ctx, format!(
                        "Hey <@{}>, it looks like the custom reaction which you added has an invalid emoji. It's been removed from the database, make sure that anything which you add is a default emoji.",
                        &reaction.user),
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
        // Give banned role to new members if they were scrims banned
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
        let _ = check_booster(&ctx, &member)
            .await
            .map_err(|err| tracing::error!("Error while checking for booster: {}", err));

        let _ = check_scrim_banned(&ctx, &member)
            .await
            .map_err(|err| tracing::error!("Error while checking for scrim banned: {}", err));
    }
}

async fn check_booster(ctx: &Context, member: &Member) -> Result<(), Box<dyn Error>> {
    let booster = member.premium_since.is_some()
        || member.permissions(ctx).map_or(false, |p| p.administrator());

    if booster {
        return Ok(());
    }

    let custom_reactions = database.fetch_custom_reactions_for(member.user.id.0);
    if !custom_reactions.is_empty() {
        // if the user's server boost runs out
        let mut embed = CreateEmbed::default();
        embed.title(format!("{}'s reaction has been removed", member.user.tag()));
        embed.description("No longer has booster and is not an administrator.");

        database.remove_custom_reaction(member.user.id.0)?;
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

async fn check_scrim_banned(ctx: &Context, member: &Member) -> Result<(), Box<dyn Error>> {
    let bans = crate::consts::DATABASE.fetch_scrim_unbans();
    let scrim_banned = bans
        .iter()
        .find(|x| !x.is_expired() && x.id == member.user.id.0);
    if let Some(scrim_banned) = scrim_banned {
        let roles = member.roles(ctx).unwrap_or_default();

        if !roles
            .iter()
            .all(|r| r.managed || r.id == crate::CONFIG.banned)
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
                .collect::<Vec<_>>();

            member.edit(&ctx, |m| m.roles(new_roles)).await?;

            if !removed_roles
                .iter()
                .all(|r| scrim_banned.roles.0.contains(&r.0))
            {
                let all_removed = [scrim_banned.roles.0.clone(), Ids::from(removed_roles).0]
                    .concat()
                    .into_iter()
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();

                crate::consts::DATABASE.modify_scrim_unban_date(
                    *member.user.id.as_u64(),
                    scrim_banned.date,
                    &Ids(all_removed),
                )?;
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
    for reaction in database.fetch_custom_reactions() {
        x.insert(reaction.trigger.to_ascii_lowercase(), reaction);
    }
    *lock = x;
}

pub async fn register_commands(ctx: &Context) -> Result<(), String> {
    #[allow(unused_mut)]
    let mut res = Ok(());
    let guild_commands = CONFIG.guild.get_application_commands(&ctx.http).await;

    for handler in &*HANDLERS {
        let name = handler.name();
        tracing::info!("Registering {}", name);
        // ignore any commands that we have already registered.
        if let Ok(ref cmds) = guild_commands {
            if cmds.iter().any(|cmd| handler.is_handler(cmd.name.clone()))
                && name.as_str() != "reload"
            {
                continue;
            }
        }
        let result = handler.register(ctx).await.map_err(|x| x.to_string());
        if let Err(ref err) = result.as_ref() {
            tracing::error!("Could not register command {}: {}", name, err);
        }
        res = res.and(result);
    }
    res
}
