use std::collections::HashMap;

use crate::commands::ban::{Ban, ScrimBan, ScrimUnban, Unban};
use crate::commands::council::Council;
use crate::commands::notes::Notes;
use crate::commands::prefabs::Prefab;
use crate::commands::timeout::Timeout;
use crate::commands::Command as _;

use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::{Message, ReactionType};
use serenity::model::gateway::Ready;
use serenity::model::id::EmojiId;
use serenity::model::interactions::Interaction;
use serenity::model::channel::MessageType;

use crate::consts::CONFIG;

type Command = Box<dyn crate::commands::Command + Send + Sync>;

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
        if msg.content.to_ascii_lowercase() == "ratio" {
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👍".into())).await {
                tracing::error!("{}", err);
            }
            if let Err(err) = msg.react(&ctx, ReactionType::Unicode("👎".into())).await {
                tracing::error!("{}", err);
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
}
