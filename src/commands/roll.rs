use crate::commands::Command;

use rand::seq::SliceRandom;
use serenity::{
    async_trait,
    model::interactions::{
        application_command::ApplicationCommandInteraction,
        InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
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
        let channel = command
            .channel_id
            .to_channel_cached(&ctx.cache)
            .await
            .unwrap()
            .guild();
        if channel.is_none()
            || channel.as_ref().unwrap().category_id.is_none()
            || !CONFIG
                .queue_categories
                .contains(&channel.unwrap().category_id.unwrap())
        {
            command
                .create_interaction_response(&ctx, |r| {
                    r.interaction_response_data(|m| {
                        m.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                            .content("This command is disabled in this channel!")
                    })
                })
                .await?;
            return Ok(());
        }

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
        let channel = channel_id
            .to_channel_cached(&ctx.cache)
            .await
            .unwrap()
            .guild()
            .unwrap();

        if channel.category_id.is_none()
            || !CONFIG
                .queue_categories
                .contains(&channel.category_id.unwrap())
        {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.content("Please join a queue before using this command.")
                })
                .await?;
            return Ok(());
        }

        let mut members = channel.members(&ctx.cache).await?;

        let user_limit: usize = channel.user_limit.unwrap_or(4).try_into().unwrap();

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

        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Roll {})
    }
}

pub struct Teams;

#[async_trait]
impl Command for Teams {
    fn name(&self) -> String {
        "teams".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Generate two teams playing scrims.")
            })
            .await?;
        Ok(())
    }
    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let channel = command
            .channel_id
            .to_channel_cached(&ctx.cache)
            .await
            .unwrap()
            .guild();
        if channel.is_none()
            || channel.as_ref().unwrap().category_id.is_none()
            || !CONFIG
                .queue_categories
                .contains(&channel.unwrap().category_id.unwrap())
        {
            command
                .create_interaction_response(&ctx, |r| {
                    r.interaction_response_data(|m| {
                        m.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                            .content("This command is disabled in this channel!")
                    })
                })
                .await?;
            return Ok(());
        }

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
        let channel = channel_id
            .to_channel_cached(&ctx.cache)
            .await
            .unwrap()
            .guild()
            .unwrap();

        if channel.category_id.is_none()
            || !CONFIG
                .queue_categories
                .contains(&channel.category_id.unwrap())
        {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.content("Please join a queue before using this command.")
                })
                .await?;
            return Ok(());
        }

        let mut members = channel.members(&ctx.cache).await?;

        let user_limit: usize = channel.user_limit.unwrap_or(4).try_into().unwrap();

        if members.len() < user_limit {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.content("This queue is not full yet.")
                })
                .await?;
            return Ok(());
        }

        members.shuffle(&mut rand::thread_rng());
        let mut is_x = true;
        let mut x = "".to_string();
        let mut y = "".to_string();
        while !members.is_empty() {
            if is_x {
                x = format!("{}\n{}", x, members.pop().unwrap().display_name());
            } else {
                y = format!("{}\n{}", y, members.pop().unwrap().display_name());
            }
            is_x = !is_x
        }

        command
            .edit_original_interaction_response(&ctx, |r| {
                r.create_embed(|e| {
                    e.title("Teams:")
                        .field("First Team", x, true)
                        .field("Second Team", y, true)
                        .color(Color::new(0x1abc9c))
                })
            })
            .await?;

        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Teams {})
    }
}
