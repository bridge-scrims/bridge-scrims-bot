use std::collections::HashMap;

use crate::commands::ban::{Ban, ScrimBan, ScrimUnban, Unban};
use crate::commands::council::Council;
use crate::commands::notes::Notes;
use crate::commands::prefabs::Prefab;
use crate::commands::purge::Purge;
use crate::commands::roll::Roll;
use crate::commands::timeout::Timeout;
use crate::commands::Command as _;

use crate::consts::CONFIG;
use rand::seq::SliceRandom;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::{Channel, Message, MessageType, ReactionType};
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
        ];
        let commands = commands
            .into_iter()
            .fold(HashMap::new(), |mut map, command| {
                map.insert(command.name(), command);
                map
            });
        Handler { commands }
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
        if msg.content.to_ascii_lowercase().contains("ratio") {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👍".into())).await {
                tracing::error!("{}", err);
            }
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👎".into())).await {
                tracing::error!("{}", err);
            }
        }

        let roll_commands: Regex = Regex::new("^!(queue|roll|captains|teams|caps|team|captain|r|swag|townhalllevel10btw|anchans|scythepro|wael|api|gez|iamanchansbitch|wasim|unicorn|noodle|Limqo|!|h|eurth|QnVubnkgR2lybA|random)").unwrap();

        if roll_commands.is_match(&msg.content)
            && CONFIG.queue_text_channels.contains(&msg.channel_id)
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

            if !CONFIG.queue_voice_channels.contains(&channel_id) {
                let _ = msg
                    .reply(&ctx, "Please join a queue before using this command.")
                    .await;
                return;
            }

            let channel = channel_id.to_channel_cached(&ctx.cache).await.unwrap();

            if let Channel::Guild(vc) = channel {
                let mut members = vc.members(&ctx.cache).await.unwrap();

                let user_limit: usize = vc.user_limit.unwrap_or(4).try_into().unwrap();

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
}
