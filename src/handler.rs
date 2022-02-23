use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::commands::ban::{Ban, ScrimBan, ScrimUnban, Unban};
use crate::commands::council::Council;
use crate::commands::notes::Notes;
use crate::commands::prefabs::Prefab;
use crate::commands::purge::Purge;
use crate::commands::reaction::{DelReaction, Reaction};
use crate::commands::roll::Roll;
use crate::commands::timeout::Timeout;
use crate::commands::Command as _;

use crate::consts::CONFIG;
use crate::consts::DATABASE as database;
use crate::db::CustomReaction;
use rand::seq::SliceRandom;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::{Message, MessageType, ReactionType};
use serenity::model::gateway::Ready;
use serenity::model::id::EmojiId;
use serenity::model::interactions::Interaction;

use serenity::model::prelude::Member;
use serenity::utils::Color;

use serenity::model::id::GuildId;
use serenity::model::user::User;

type Command = Box<dyn crate::commands::Command + Send + Sync>;
use regex::Regex;

pub struct Handler {
    commands: HashMap<String, Command>,
    reactions: Arc<Mutex<HashMap<String, CustomReaction>>>,
}

impl Handler {
    pub fn new() -> Handler {
        let commands: Vec<Command> = vec![
            Council::new(),
            Notes::new(),
            Prefab::new(),
            Timeout::new(),
            Ban::new(),
            Unban::new(),
            ScrimBan::new(),
            ScrimUnban::new(),
            Roll::new(),
            Purge::new(),
            Reaction::new(),
            DelReaction::new(),
        ];
        let commands = commands
            .into_iter()
            .fold(HashMap::new(), |mut map, command| {
                map.insert(command.name(), command);
                map
            });

        Handler {
            commands,
            reactions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, data: Ready) {
        tracing::info!("Connected to discord as {}", data.user.tag());
        for (name, command) in &self.commands {
            tracing::info!("Registering {}", name);
            if let Err(err) = command.register(&_ctx).await {
                tracing::error!("Could not register command {}: {}", name, err);
            }
        }
        tokio::spawn(update_reactions(self.reactions.clone()));
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command_interaction) = interaction {
            if let Some(command) = self.commands.get(&command_interaction.data.name) {
                if let Err(err) = command.run(&ctx, &command_interaction).await {
                    tracing::error!("{} command failed: {}", command.name(), err);
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

        let roll_commands: Regex = Regex::new("^!(queue|roll|captains|teams|caps|team|captain|r|swag|townhalllevel10btw|anchans|scythepro|wael|api|gez|iamanchansbitch|wasim|unicorn|noodle|Limqo|!|h|eurth|QnVubnkgR2lybA|random)").unwrap();

        let channel = msg
            .channel_id
            .to_channel_cached(&ctx.cache)
            .await
            .unwrap()
            .guild();
        if roll_commands.is_match(&msg.content)
            && channel.as_ref().is_some()
            && channel.as_ref().unwrap().category_id.is_some()
            && CONFIG
                .queue_categories
                .contains(&channel.unwrap().category_id.unwrap())
        {
            let member = msg.author.clone();

            let guild = CONFIG.guild.to_guild_cached(&ctx.cache).await.unwrap();

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
                .await
                .unwrap()
                .guild()
                .unwrap();
            if !CONFIG
                .queue_categories
                .contains(&channel.category_id.unwrap())
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
                            .field("First Captain", members[0].display_name(), true)
                            .field("Second Captain", members[1].display_name(), true)
                            .color(Color::new(0x1abc9c))
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
        if msg.channel_id.as_u64() == CONFIG.polls.as_u64() {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("✅".into())).await {
                tracing::error!("{}", err);
            }
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("❌".into())).await {
                tracing::error!("{}", err);
            }
        }
        if msg.kind == MessageType::MemberJoin {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👋".into())).await {
                tracing::error!("{}", err);
            }
        }
        let reactions = self.reactions.lock().await;
        if let Some(reaction) = reactions.get(&msg.content.to_ascii_lowercase()) {
            if let Err(err) = msg
                .react(&ctx, ReactionType::Unicode(reaction.emoji.clone()))
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
                            &reaction.user)
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

    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, _member: Member) {
        if let Err(err) = CONFIG.member_count.update(ctx, guild_id).await {
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

    async fn guild_member_update(&self, ctx: Context, _old_data: Option<Member>, user: Member) {
        let mut x = false;
        for role in user.roles(&ctx.cache).await.unwrap() {
            if role.tags.premium_subscriber {
                x = true;
            }
        }
        if x && !database
            .fetch_custom_reactions_for(user.user.id.0)
            .is_empty()
        {
            // if the user's server boost runs out

            if let Err(err) = database.remove_custom_reaction(user.user.id.0) {
                tracing::error!("Error when updating database: {}", err);
            }
            // be sure to update the other thing
            update(self.reactions.clone()).await;
        }
    }
}

async fn update_reactions(m: Arc<Mutex<HashMap<String, CustomReaction>>>) {
    loop {
        update(m.clone()).await;
        tokio::time::sleep(Duration::from_secs(60 * 60 * 12)).await;
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
