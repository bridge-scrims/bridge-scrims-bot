use std::{sync::Arc, collections::HashMap, slice::Iter};
use tokio::{time::{sleep, Duration}, sync::MutexGuard};
use rand::seq::SliceRandom;
use futures::StreamExt;
use tokio::sync::Mutex;
use regex::Regex;
use serenity::{
    async_trait,
    utils::Color,
    model::{application::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType, MessageFlags,
    }, prelude::{ChannelId, ChannelType, GuildChannel, component::ButtonStyle, Guild, Message, UserId}},
    prelude::{Context, Mentionable},
    builder::{CreateEmbed, CreateInteractionResponseData}
};

use crate::commands::Command;
use crate::consts::CONFIG;

const GAME_TYPES: [&str; 4] = ["1v1", "2v2", "3v3", "4v4"];

async fn verify_correct_channel(ctx: &Context, command: &ApplicationCommandInteraction) -> bool {
    let queue_categories = CONFIG.queue_categories.values().cloned().flatten().collect::<Vec<_>>();
    let channel = command.channel_id.to_channel(&ctx).await.map(|c| c.guild());
    if !channel.is_ok() || !channel.as_ref().unwrap().is_some()
        || !queue_categories
            .contains(&channel.unwrap().unwrap().parent_id.unwrap_or_default())
    {
        let _ = command
            .create_interaction_response(&ctx, |r| {
                r.interaction_response_data(|m| {
                    m.flags(MessageFlags::EPHEMERAL)
                        .content("This command is disabled in this channel!")
                })
            }).await;
        return false;
    }
    true
}

async fn verify_correct_vc(ctx: &Context, command: &ApplicationCommandInteraction, guild: &Guild) -> Option<GuildChannel> {
    let member = command.member.as_ref().unwrap();
    let voice_state = guild.voice_states.get(&member.user.id);
    if voice_state.is_none() || voice_state.unwrap().channel_id.is_none() {
        let _ = command
            .edit_original_interaction_response(&ctx, |r| {
                r.content("Please join a queue before using this command.")
            }).await;
        return None;
    }
    let vc_id = voice_state.unwrap().channel_id.unwrap();
    Some(vc_id.to_channel_cached(&ctx.cache).unwrap().guild().unwrap())
}

async fn verify_correct_vc_members(ctx: &Context, command: &ApplicationCommandInteraction, vc: &GuildChannel) -> Option<Vec<UserId>> {
    let members = vc_members(&ctx, &vc);
    let user_limit: usize = vc.user_limit.unwrap_or(2).try_into().unwrap();
    if members.len() >= user_limit {
        let _ = command
            .edit_original_interaction_response(&ctx, |r| {
                r.content("This queue is not full yet.")
            }).await;
        return None;
    }
    Some(members)
}

fn vc_members_count(guild: &Guild, vc_id: ChannelId) -> usize {
    guild.voice_states.values().filter(|v| v.channel_id.unwrap_or_default() == vc_id).count()
}

fn vc_members(ctx: &Context, vc: &GuildChannel) -> Vec<UserId> {
    if let Some(guild) = vc.guild(&ctx.cache) {
        return guild.voice_states.values()
            .filter(|v| v.channel_id.unwrap_or_default() == vc.id)
            .map(|v| &v.member).filter(|m| m.is_some()).map(|m| m.as_ref().unwrap().user.id)
            .collect::<Vec<_>>();
    }
    Vec::new()
}

pub struct Teams {
    reserved_calls: Arc<Mutex<HashMap<ChannelId, u64>>>
}

impl Teams {

    async fn remove_reservation(reserved_calls: Arc<Mutex<HashMap<ChannelId, u64>>>, channel_id: ChannelId) {
        sleep(Duration::from_secs(60)).await;
        let mut reserved_calls = reserved_calls.lock().await;
        reserved_calls.remove(&channel_id);
    }

    fn find_team_channel(
        &self, 
        reserved_calls: &mut MutexGuard<HashMap<ChannelId, u64>>, 
        team_channels: &Vec<GuildChannel>, 
        game_rank_index: usize, 
        game_type: &str, 
        roll_id: u64
    ) -> Option<GuildChannel> {
        for (i, (_rank, categories)) in CONFIG.queue_categories.iter().enumerate().collect::<Vec<_>>().iter().rev() {
            if i > &game_rank_index {
                // e.g. if this command came from a private queue then premium team channels are ignored
                continue;
            }
    
            let team_channel =  team_channels.iter()
                .filter(|c| categories.contains(c.parent_id.unwrap_or_default().as_ref()))
                .filter(|c| !GAME_TYPES.iter().any(|t| c.name().contains(t)) || c.name().contains(game_type))
                .next();
            if let Some(team_channel) = team_channel {
                reserved_calls.insert(team_channel.id, roll_id);
                tokio::spawn(Teams::remove_reservation(self.reserved_calls.clone(), team_channel.id));
                return Some(team_channel.to_owned());
            }
        }
        None
    }
    
    async fn build_team_embed(ctx: &Context, title: &str, color: Color, members: Iter<'_, UserId>, team_call: Option<&GuildChannel>) -> CreateEmbed {
        let mut embed = CreateEmbed::default();
        embed.title(title);
        embed.color(color);
    
        let mentions = members.map(|m| m.mention().to_string()).collect::<Vec<_>>().join(" ");
        embed.field("Members", mentions, false);
        if team_call.is_some() {
            let invite = team_call.unwrap().create_invite(ctx, |i| i).await;
            if invite.is_ok() {
                embed.field("Team Call", format!("{} *[click to join]({})*", team_call.unwrap().mention(), invite.unwrap().url()), false);
            }
            
        }
        embed
    }

    async fn build_teams_payload<'a>(ctx: &Context, voice_channel: &GuildChannel, calls: &Vec<Option<GuildChannel>>) -> Option<CreateInteractionResponseData<'a>> {
        let team_size = (voice_channel.user_limit.unwrap_or(4) as f32 / 2 as f32).floor() as usize;
        let mut members = vc_members(&ctx, voice_channel);
        if team_size < 1 || members.len() < team_size*2 {
            return None;
        }
        
        members.shuffle(&mut rand::thread_rng());

        let team1 = Teams::build_team_embed(
            &ctx, "First Team", Color::from_rgb(70, 55, 86), 
            members[..team_size].iter(), calls.get(0).unwrap().as_ref()
        ).await;
        let team2 = Teams::build_team_embed(
            &ctx, "Second Team", Color::from_rgb(161, 79, 80), 
            members[team_size..].iter(), calls.get(1).unwrap().as_ref()
        ).await;

        let mut response_data = CreateInteractionResponseData::default();
        response_data
            .content(format!("**For {}:**", voice_channel.mention()))
            .add_embed(team1).add_embed(team2)
            .components(|c| c.create_action_row(
                |r| r.create_button(
                    |b| b
                        .custom_id("REROLL")
                        .label("Reroll")
                        .style(ButtonStyle::Primary)
                        .emoji('🎲')
                )
            ));
        Some(response_data)
    }

    async fn collect_and_handle_reroll_components(ctx: &Context, resp: Message, voice_channel: &GuildChannel, calls: Vec<Option<GuildChannel>>) {
        
        let mut collector = resp.await_component_interactions(&ctx)
            .timeout(Duration::from_secs(60))
            .filter(|i| i.data.custom_id == "REROLL")
            .build();

        'collector:
        while let Some(interaction) = collector.next().await {
            'valid: {
                if let Some(g) = interaction.guild_id {
                    if let Some(g) = g.to_guild_cached(&ctx) {
                        if let Some(v) = g.voice_states.get(&interaction.user.id) {
                            if let Some(v) = v.channel_id {
                                if v == voice_channel.id 
                                    && (vc_members_count(&g, v) as u64) >= voice_channel.user_limit.unwrap_or(2)
                                {
                                    break 'valid;
                                }
                            }
                        }
                    }
                }
                continue 'collector;
            };

            if let Some(response_data) = Teams::build_teams_payload(&ctx, voice_channel, &calls).await {
                let _ = interaction
                    .create_interaction_response(&ctx,
                        |r| r
                            .kind(InteractionResponseType::UpdateMessage)
                            .interaction_response_data(|r| {
                                r.0 = response_data.0;
                                r
                            })
                    ).await;
            }
        }
    }

}

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

        if !verify_correct_channel(&ctx, &command).await {
            return Ok(());
        }

        command.create_interaction_response(&ctx, |r| {
            r.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        }).await?;

        let guild = command.guild_id.unwrap().to_guild_cached(&ctx.cache).unwrap();
        let voice_channel = match verify_correct_vc(&ctx, &command, &guild).await {
            Some(v) => v,
            None => return Ok(())
        };

        let mut members = match verify_correct_vc_members(&ctx, &command, &voice_channel).await {
            Some(v) => v,
            None => return Ok(())
        };

        let guild_channels = ctx.cache.guild_channels(command.guild_id.unwrap()).unwrap_or_default();
        let reserved_calls = self.reserved_calls.lock().await;
        let tc_re = Regex::new(r"team.+\d+").unwrap();
        
        let mut team_channels = Vec::new();
        for c in guild_channels.iter() {
            if c.kind == ChannelType::Voice
                && !reserved_calls.contains_key(&c.id)
                && vc_members_count(&guild, c.id) == 0
                && tc_re.is_match(&c.name().to_lowercase()) 
            {
                team_channels.push(c.value().clone());
            }
        }
        drop(reserved_calls);
        team_channels.sort_by(|a, b| a.position.cmp(&b.position));
    
        let game_type = GAME_TYPES.iter().cloned().find(|v| voice_channel.name().contains(v)).unwrap_or("");
        let game_rank_index = CONFIG.queue_categories.values()
            .position(|v| v.contains(&voice_channel.parent_id.unwrap_or_default()))
            .unwrap_or_default();
        
        members.shuffle(&mut rand::thread_rng());

        let mut reserved_calls = self.reserved_calls.lock().await;
        let calls = vec![0; 2].iter()
            .map(|_| self.find_team_channel(&mut reserved_calls, &team_channels, game_rank_index, game_type, command.id.0)).collect::<Vec<_>>();
        drop(reserved_calls);

        if let Some(payload) = Teams::build_teams_payload(&ctx, &voice_channel, &calls).await {
            let resp = command
                .edit_original_interaction_response(&ctx, |r| {
                    r.0 = payload.0;
                    r
                }).await;

            if let Ok(resp) = resp {
                Teams::collect_and_handle_reroll_components(&ctx, resp, &voice_channel, calls).await;
            }
        }

        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Teams {
            reserved_calls: Arc::new(Mutex::new(HashMap::new()))
        })
    }
}
