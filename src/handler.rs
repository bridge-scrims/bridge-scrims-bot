use std::collections::HashMap;

use crate::commands::Command as _;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::{Message, ReactionType};
use serenity::model::gateway::Ready;
use serenity::model::id::EmojiId;
use serenity::model::interactions::Interaction;
use serenity::model::prelude::Member;
use crate::commands::council::Council;
use crate::commands::purge::Purge;
use crate::consts::GUILD;
use serenity::model::prelude::GuildId;
use serenity::model::prelude::RoleId;
use serenity::futures::StreamExt;
use serenity::model::prelude::ChannelId;
type Command = Box<dyn crate::commands::Command + Send + Sync>;

pub struct Handler {
    commands: HashMap<String, Command>,
}

impl Handler {
    pub fn new() -> Handler {
        let commands: Vec<Command> = vec![Council::new(), Purge::new()];
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
                let channel = ChannelId(905110419742552086);

        let button = match channel.send_message(_ctx, |m|{

            m.content("_ _".to_string());
            m.components(|c|{
                c.create_action_row(|a|{
                    a.create_button(|b|{
                        b.style(ButtonStyle::Success);
                        b.label("Create A Ticket!");
                        b.custom_id("ticket_button")
                    })             
                })
            })


        }).await {
            Ok(msg) => msg,
            Err(e) => {
                println!("There was an error: {}", e);
                return;
            }
        };

        while let Some(interaction) = serenity::CollectComponentInteraction::new(&ctx)
        .channel_id(channel)
        .message_id(button.id)
        .timeout()
        .await 
            {
               let ticket_channel = GuildId(901821870582665247).create_channel(&_ctx, |c| 
                
                
                
                c.name("my-test-channel").kind(ChannelType::Text))
                
               .await;
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
            .replace(" ", "")
            .contains("shmill")
        {
            if let Err(err) = msg
                .react(
                    &ctx,
                    GUILD
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
    }



    #[allow(unused_variables)]
    #[allow(unused_must_use)]
    #[allow(unused_assignments)]
async fn guild_member_addition(&self, ctx: Context, _guild_id: GuildId, _member: Member) {
    
    let channel: ChannelId = ChannelId(901821871056629803);
    let member_role_id = 904856692787937400;
    let guild = GuildId(901821870582665247);
    let mut counter = 0;

    let _members = guild
        .members_iter(&ctx.http)
        .filter(|u| {
            let has_role = if let Ok(u) = u {
                u.roles.contains(&RoleId(member_role_id))
            } else {
                false
            };
            async move { has_role }
        }).for_each(|_| async move {
            counter += 1;
        }).await;

    channel.edit(&ctx.http, |c| c.name(counter.to_string())).await.unwrap();
}

    
}
