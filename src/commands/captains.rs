use rand::seq::SliceRandom;
use serenity::{
    async_trait, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::prelude::*,
};

use crate::consts::CONFIG;
use bridge_scrims::{discord_util::vc_members, interaction::*};

pub struct CaptainsCommand;

#[async_trait]
impl InteractionHandler for CaptainsCommand {
    fn name(&self) -> String {
        "captains".to_string()
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Generate two team captains")
            })
            .await?;
        Ok(())
    }

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        let channel = command.channel_id.to_channel(&ctx).await?.guild();
        if !channel.map_or(false, |c| {
            c.parent_id.map_or(false, |p| {
                CONFIG.rank_queue_categories.concat().contains(&p)
            })
        }) {
            Err(ErrorResponse::message(
                "This command is disabled in this channel!",
            ))?;
        }

        let vc = command_vc(ctx, &command.guild_id, &command.user.id)?;
        let mut members = vc_members(ctx, &vc);

        if vc
            .user_limit
            .map_or(false, |limit| members.len() < limit as usize)
        {
            Err(ErrorResponse::message("This queue is not full yet."))?;
        }

        members.shuffle(&mut rand::thread_rng());

        command.create_interaction_response(ctx, |resp| 
            resp.kind(interaction::InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|d|
                    d.embed(|e| {
                        e.author(|a| {
                            a.name(vc.name())
                                .icon_url("https://cdn.discordapp.com/attachments/1075184074718707722/1131610023622086706/766c86e6244395ea36c530a7a4f27242.png")
                        })
                        .color(0x5CA3F5)
                        .field("First Captain", members.get(0).map_or(String::from("None"), |m| m.mention().to_string()), true)
                        .field("Second Captain",  members.get(1).map_or(String::from("None"), |m| m.mention().to_string()), true)
                        .footer(|f| f.text("Captains should take turns selecting team members until two teams are formed."))
                    }),
                )
        ).await?;

        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(CaptainsCommand {})
    }
}

pub fn command_vc(
    ctx: &Context,
    guild_id: &Option<GuildId>,
    user_id: &UserId,
) -> crate::Result<GuildChannel> {
    if let Some(guild) = guild_id {
        if let Some(guild) = guild.to_guild_cached(ctx) {
            if let Some(voice_state) = guild.voice_states.get(user_id) {
                if let Some(vc) = voice_state.channel_id {
                    if let Some(vc) = vc.to_channel_cached(ctx) {
                        if let Some(vc) = vc.guild() {
                            return Ok(vc);
                        }
                    }
                }
            }
        }
    }
    
    Err(ErrorResponse::message(
        "Please join a queue before using this command.",
    ))?
}
