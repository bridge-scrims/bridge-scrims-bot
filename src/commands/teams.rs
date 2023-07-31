use rand::seq::SliceRandom;
use regex::Regex;

use serenity::{
    async_trait, builder::CreateInteractionResponse, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::application::interaction::message_component::MessageComponentInteraction,
    model::prelude::*,
};

use crate::consts::CONFIG;
use bridge_scrims::discord_util::vc_members;
use bridge_scrims::interaction::respond::RespondableInteraction;
use bridge_scrims::interaction::*;

lazy_static::lazy_static! {
    static ref GAME_MODES: [String; 4] = ["1v1", "2v2", "3v3", "4v4"].map(String::from);
    static ref RANK_QUEUE_CATEGORIES: &'static Vec<Vec<ChannelId>> = &CONFIG.rank_queue_categories;
    static ref TEAM_CALL_REGEX: Regex = Regex::new(r"team.+\d+").unwrap();
    static ref USER_MENTION_REGEX: Regex = Regex::new(r"<@(\d+)>").unwrap();
}

#[non_exhaustive]
struct TeamActions;
impl TeamActions {
    pub const REROLL: &str = "REROLL";
    pub const JOIN_CALL: &str = "JOIN_CALL";
}

pub struct TeamsCommand;

impl TeamsCommand {
    fn build_teams_response<'a>(
        &self,
        ctx: &Context,
        vc: &GuildChannel,
    ) -> CreateInteractionResponse<'a> {
        let mut members = vc_members(ctx, vc);
        members.shuffle(&mut rand::thread_rng());

        let middle_index = members.len() / 2;
        let (team1, team2) = (&members[..middle_index], &members[middle_index..]);

        let mut response = CreateInteractionResponse::default();
        response.interaction_response_data(|d| {
            d.embed(|e| {
                e.author(|a| {
                    a.name(vc.name())
                        .icon_url("https://cdn.discordapp.com/attachments/1075184074718707722/1131610023622086706/766c86e6244395ea36c530a7a4f27242.png")
                })
                .color(0x5CA3F5)
                .field("First Team", team_field_value(team1), true)
                .field("Second Team", team_field_value(team2), true)
            })
            .components(|c| {
                c.create_action_row(|a| {
                    a.create_button(|b| {
                        b.custom_id(self.button_custom_id(vec![
                            TeamActions::REROLL,
                            vc.id.to_string().as_str(),
                        ]))
                        .label("Reroll")
                        .style(component::ButtonStyle::Primary)
                        .emoji('🎲')
                    })
                    .create_button(|b| {
                        b.custom_id(self.button_custom_id(vec![
                            TeamActions::JOIN_CALL,
                            vc.id.to_string().as_str(),
                        ]))
                        .label("Join Team Call")
                        .style(component::ButtonStyle::Primary)
                        .emoji('🔊')
                    })
                })
            })
        });
        response
    }

    fn button_custom_id(&self, args: Vec<&str>) -> String {
        format!("{}:{}", self.name(), args.join(":"))
    }
}

fn team_field_value(team: &[UserId]) -> String {
    team.iter()
        .map(|u| u.mention().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[async_trait]
impl InteractionHandler for TeamsCommand {
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

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        let channel = command.channel_id.to_channel(&ctx).await?.guild();
        if !channel.map_or(false, |c| {
            c.parent_id
                .map_or(false, |p| RANK_QUEUE_CATEGORIES.concat().contains(&p))
        }) {
            Err(ErrorResponse::message(
                "This command is disabled in this channel!",
            ))?;
        }

        let vc = command_vc(ctx, &command.guild_id, &command.user.id)?;
        let members = vc_members(ctx, &vc);

        if vc
            .user_limit
            .map_or(false, |limit| members.len() < limit as usize)
        {
            Err(ErrorResponse::message("This queue is not full yet."))?;
        }

        let mut resp = self.build_teams_response(ctx, &vc);
        resp.kind(interaction::InteractionResponseType::ChannelMessageWithSource);
        command.create_response(ctx, resp).await?;

        Ok(None)
    }

    async fn handle_component(
        &self,
        ctx: &Context,
        command: &MessageComponentInteraction,
        args: &[&str],
    ) -> InteractionResult {
        let action = args.first().unwrap();
        let expected_queue = args.get(1).unwrap();

        let queue = command_vc(ctx, &command.guild_id, &command.user.id)?;
        if queue.id.to_string() != *expected_queue {
            Err(ErrorResponse::message(
                "You are not in the correct queue to do this.",
            ))?;
        }

        if *action == TeamActions::REROLL {
            if queue.user_limit.map_or(false, |limit| {
                vc_members(ctx, &queue).len() < limit as usize
            }) {
                Err(ErrorResponse::message(
                    "This queue channel is no longer full.",
                ))?;
            }

            let mut resp = self.build_teams_response(ctx, &queue);
            resp.kind(interaction::InteractionResponseType::UpdateMessage);
            command.create_response(ctx, resp).await?;
        } else if *action == TeamActions::JOIN_CALL {
            let team = command
                .message
                .embeds
                .first()
                .unwrap()
                .fields
                .iter()
                .map(|f| {
                    USER_MENTION_REGEX
                        .captures_iter(&f.value)
                        .map(|captures| {
                            UserId(captures.get(1).unwrap().as_str().parse::<u64>().unwrap())
                        })
                        .collect::<Vec<_>>()
                })
                .find(|t| t.contains(&command.user.id))
                .unwrap_or_default();

            let call = find_team_call(ctx, &queue, &team);
            if call.is_none() {
                Err(ErrorResponse::message(
                    "No suitable team call could be found for you.",
                ))?;
            }

            command
                .member
                .as_ref()
                .unwrap()
                .move_to_voice_channel(ctx, call.unwrap())
                .await?;

            command
                .create_interaction_response(ctx, |r| {
                    r.kind(interaction::InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|d| {
                            d.content(format!(
                                ":white_check_mark:  Moved you to {}",
                                call.unwrap().mention()
                            ))
                            .ephemeral(true)
                        })
                })
                .await?;
        }

        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(TeamsCommand {})
    }
}

fn find_team_call(ctx: &Context, vc: &GuildChannel, team: &[UserId]) -> Option<ChannelId> {
    let game_mode = GAME_MODES.iter().find(|x| vc.name().contains(x.as_str()));
    if let Some(game_mode) = game_mode {
        let game_rank_idx = vc
            .parent_id
            .and_then(|parent_id| {
                RANK_QUEUE_CATEGORIES
                    .iter()
                    .position(|r| r.contains(&parent_id))
            })
            .unwrap_or(0);

        return get_rank_team_call(ctx, vc.guild_id, game_mode.as_str(), game_rank_idx, team);
    }
    None
}

fn get_rank_team_call(
    ctx: &Context,
    guild: GuildId,
    game_mode: &str,
    game_rank_idx: usize,
    team: &[UserId],
) -> Option<ChannelId> {
    let mut rank_channels = RANK_QUEUE_CATEGORIES
        .iter()
        .take(game_rank_idx + 1)
        .rev()
        .map(|categories| {
            ctx.cache
                .categories()
                .iter()
                .filter(|c| c.guild_id == guild)
                .filter(|c| categories.contains(&c.id))
                .flat_map(|cat| {
                    ctx.cache
                        .guild_channels(guild)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|x| x.1)
                        .filter(move |x| x.parent_id == Some(cat.id))
                })
                .filter(|vc| vc.kind == ChannelType::Voice)
                .filter(|vc| TEAM_CALL_REGEX.is_match(vc.name().to_lowercase().as_str()))
                .filter(|vc| {
                    !GAME_MODES.iter().any(|x| vc.name().contains(x.as_str()))
                        || vc.name().contains(game_mode)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    rank_channels
        .iter_mut()
        .for_each(|x| x.sort_by_key(|vc| vc.position));

    let channels = rank_channels.concat();

    let from_team = channels.iter().find(|vc| {
        let members = vc_members(ctx, vc);
        vc.user_limit
            .map_or(true, |limit| members.len() < limit as usize)
            && team.iter().any(|t| members.contains(t))
    });

    from_team
        .or_else(|| channels.iter().find(|vc| vc_members(ctx, vc).is_empty()))
        .map(|vc| vc.id)
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
