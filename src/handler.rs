use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use regex::Regex;
use serenity::async_trait;
use serenity::builder::CreateEmbed;
use serenity::client::{Context, EventHandler};
use serenity::model::application::interaction::{Interaction, MessageFlags};
use serenity::model::channel::{Message, MessageType, ReactionType};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::id::{ChannelId, EmojiId};
use serenity::model::prelude::Member;
use serenity::model::user::User;
use serenity::prelude::Mentionable;
use serenity::utils::Color;
use tokio::sync::Mutex;

use crate::commands::ban::{Ban, ScrimBan};
use crate::commands::close::Close;
use crate::commands::council::Council;
use crate::commands::freeze::Freeze;
use crate::commands::list_bans::ListBans;
use crate::commands::logtime::LogTime;
use crate::commands::notes::Notes;
use crate::commands::ping::Ping;
use crate::commands::prefabs::Prefab;
use crate::commands::purge::Purge;
use crate::commands::reaction::{DelReaction, ListReactions, Reaction};
use crate::commands::reload::Reload;
use crate::commands::roll::{Roll, Teams};
use crate::commands::screenshare::Screenshare;
use crate::commands::screensharers::Screensharers;
use crate::commands::ticket::Ticket;
use crate::commands::timeout::Timeout;
use crate::commands::unban::{ScrimUnban, Unban};
use crate::commands::unfreeze::Unfreeze;
use crate::commands::Command as _;
use crate::consts::CONFIG;
use crate::consts::DATABASE as database;
use crate::db::CustomReaction;

type Command = Box<dyn crate::commands::Command>;

lazy_static! {
    pub static ref COMMANDS: Vec<Command> = vec![
        Council::new(),
        Notes::new(),
        Prefab::new(),
        Timeout::new(),
        Ban::new(),
        Unban::new(),
        ScrimBan::new(),
        ScrimUnban::new(),
        Roll::new(),
        Teams::new(),
        Purge::new(),
        Reaction::new(),
        DelReaction::new(),
        ListReactions::new(),
        Screenshare::new(),
        Close::new(),
        Freeze::new(),
        Unfreeze::new(),
        Ticket::new(),
        ListBans::new(),
        Screensharers::new(),
        Reload::new(),
        Ping::new(),
        LogTime::new(),
    ];
}

pub struct Handler {
    init: Mutex<bool>,
    reactions: Arc<Mutex<HashMap<String, CustomReaction>>>,
}

impl Handler {
    pub fn new() -> Handler {
        Handler {
            init: Mutex::new(false),
            reactions: Arc::new(Mutex::new(HashMap::new())),
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
            for command in &*COMMANDS {
                command.init(&ctx).await
            }
            tokio::spawn(update_reactions(self.reactions.clone()));
        }
        // Errors are already handled
        let _ = register_commands(&ctx).await;
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command_interaction) = interaction {
            if let Some(command) = COMMANDS
                .iter()
                .find(|x| x.is_command(command_interaction.data.name.clone()))
            {
                if let Err(err) = command.run(&ctx, &command_interaction).await {
                    tracing::error!("{} command failed: {}", command.name(), err);
                }
                if command.name().contains("reaction") {
                    update(self.reactions.clone()).await;
                }
                if command.name().contains("reload") {
                    let res = register_commands(&ctx).await;
                    let response = command_interaction
                        .create_followup_message(&ctx.http, |resp| {
                            resp.content(match res {
                                Ok(_) => "Successfully reloaded!".to_string(),
                                Err(e) => format!("Reloading failed: {}", e),
                            })
                            .flags(MessageFlags::EPHEMERAL)
                        })
                        .await;
                    if let Err(e) = response {
                        tracing::error!("Reloading failed: {}", e);
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
        let reactions = self.reactions.lock().await;
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
        if let Err(err) = CONFIG.member_count.update(ctx, member.guild_id).await {
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
        if let Err(err) = CONFIG.member_count.update(ctx, guild_id).await {
            tracing::error!("Error when updating member count: {}", err)
        }
    }

    async fn guild_member_update(&self, ctx: Context, _old_data: Option<Member>, mut user: Member) {
        let mut x = false;

        for role in user.roles(&ctx.cache).unwrap() {
            if role.tags.premium_subscriber || role.id == CONFIG.staff {
                x = true;
            }
        }
        let custom_reactions = database.fetch_custom_reactions_for(user.user.id.0);
        if !x && !custom_reactions.is_empty() {
            // if the user's server boost runs out
            let mut embed = CreateEmbed::default();
            embed.title(format!("{}'s reaction has been removed", user.user.tag()));
            embed.description("No longer has booster or staff role.");

            if let Err(err) = database.remove_custom_reaction(user.user.id.0) {
                tracing::error!("Error when updating database: {}", err);
            }
            if let Err(err) = CONFIG
                .reaction_logs
                .send_message(&ctx, |msg| msg.set_embed(embed.clone()))
                .await
            {
                tracing::error!("Error when sending message: {}", err);
            }
            // be sure to update the other thing
            update(self.reactions.clone()).await;
        }

        if !x {
            for role in user.roles(&ctx.cache).unwrap() {
                for a in &CONFIG.color_roles {
                    if &role.id == a {
                        if let Err(err) = user.remove_role(&ctx.http, a).await {
                            tracing::error!("{}", err);
                        }
                    }
                }
            }
        }

        let mut has_banned = false;
        let mut has_member = false;
        let mut has_unverified = false;
        for role in user.roles(&ctx.cache).unwrap() {
            if role.id == CONFIG.member_role {
                has_member = true;
            }
            if role.id == CONFIG.unverified_role {
                has_unverified = true;
            }
            if role.id == CONFIG.banned {
                has_banned = true;
            }
        }
        if !user.roles(&ctx.cache).unwrap().is_empty()
            && !has_banned
            && !has_unverified
            && !has_member
            && crate::consts::DATABASE.fetch_freezes_for(user.user.id.0).is_none()
        {
            if let Err(err) = user.add_role(&ctx.http, CONFIG.member_role).await {
                tracing::error!("{}", err);
            }
        }
    }
}

async fn update_reactions(m: Arc<Mutex<HashMap<String, CustomReaction>>>) {
    loop {
        tracing::info!("Updating reactions...");
        update(m.clone()).await;
        tokio::time::sleep(Duration::from_secs(60 * 60 * 2)).await;
    }
}

async fn update(m: Arc<Mutex<HashMap<String, CustomReaction>>>) {
    let mut lock = m.lock().await;
    let mut x = HashMap::new();
    for reaction in database.fetch_custom_reactions() {
        x.insert(reaction.trigger.to_ascii_lowercase(), reaction);
    }
    *lock = x;
}

async fn register_commands(ctx: &Context) -> Result<(), String> {
    #[allow(unused_mut)]
    let mut res = Ok(());
    let guild_commands = CONFIG.guild.get_application_commands(&ctx.http).await;

    for command in &*COMMANDS {
        let name = command.name();
        tracing::info!("Registering {}", name);
        // ignore any commands that we have already registered.
        if let Ok(ref cmds) = guild_commands {
            if cmds.iter().any(|cmd| command.is_command(cmd.name.clone()))
                && name.as_str() != "reload"
            {
                continue;
            }
        }
        // let result = command.register(ctx).await.map_err(|x| x.to_string());
        // if let Err(ref err) = result.as_ref() {
        //     tracing::error!("Could not register command {}: {}", name, err);
        // }
        // res = res.and(result);
    }
    res
}
