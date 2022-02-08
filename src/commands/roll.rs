use crate::commands::Command;

use rand::seq::SliceRandom;
use serenity::{
    async_trait,
    model::{
        channel::Channel,
        interactions::{
            application_command::ApplicationCommandInteraction, InteractionResponseType,
        },
    },
    prelude::Context,
    utils::Color,
};

use crate::consts::CONFIG;

pub struct Roll;

#[async_trait]
impl Command for Roll {
    fn name(&self) -> String {
        "roll".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Generate two captains for playing scrims.")
            })
            .await?;
        Ok(())
    }
    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        command
            .create_interaction_response(&ctx, |r| {
                r.kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        let member = command.member.as_ref().unwrap();

        let guild = CONFIG.guild.to_guild_cached(&ctx.cache).await.unwrap();

        let voice_state = guild.voice_states.get(&member.user.id);

        if voice_state.is_none() || voice_state.unwrap().channel_id.is_none() {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.content("Please join a queue before using this command.")
                })
                .await?;
            return Ok(());
        }

        let channel_id = voice_state.unwrap().channel_id.unwrap();

        if !CONFIG.queue_channels.contains(&channel_id) {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.content("Please join a queue before using this command.")
                })
                .await?;
            return Ok(());
        }

        let channel = channel_id.to_channel_cached(&ctx.cache).await.unwrap();

        if let Channel::Guild(vc) = channel {
            let mut members = vc.members(&ctx.cache).await?;

            let user_limit: usize = vc.user_limit.unwrap_or(4).try_into().unwrap();

            if members.len() < user_limit {
                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.content("This queue is not full yet.")
                    })
                    .await?;
                return Ok(());
            }

            members.shuffle(&mut rand::thread_rng());

            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.create_embed(|e| {
                        e.title("Team Captains:")
                            .field("First Captain", members[0].display_name(), true)
                            .field("Second Captain", members[1].display_name(), true)
                            .color(Color::new(0x1abc9c))
                    })
                })
                .await?;
        }

        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Roll {})
    }
}
