use std::collections::HashMap;

use crate::commands::Command as _;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::interactions::Interaction;

use crate::commands::council::Council;
use crate::commands::suggest::Suggestion;

type Command = Box<dyn crate::commands::Command + Send + Sync>;

pub struct Handler {
    commands: HashMap<String, Command>,
}

impl Handler {
    pub fn new() -> Handler {
        let commands: Vec<Command> = vec![Council::new(), Suggestion::new()];
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
}