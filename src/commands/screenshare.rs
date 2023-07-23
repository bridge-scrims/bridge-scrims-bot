use std::time::Duration;
use tokio::time::sleep;

use serenity::{
    async_trait,
    builder::{CreateInteractionResponseData, CreateMessage},
    client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::prelude::*,
};

use bridge_scrims::interaction::*;

use crate::consts::DATABASE;

use super::close;

lazy_static::lazy_static! {
    pub static ref ALLOW_PERMS: Permissions = Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY | Permissions::SEND_MESSAGES;
    pub static ref DENY_PERMS: Permissions = Permissions::empty();
}

pub struct Screenshare;

#[async_trait]
impl InteractionHandler for Screenshare {
    fn name(&self) -> String {
        String::from("screenshare")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::CONFIG
            .guild
            .create_application_command(&ctx.http, |command| {
                command
                    .name(self.name())
                    .description("Creates a screenshare ticket.")
                    .create_option(|option| {
                        option
                            .name("user")
                            .description("The Discord user that should be screenshared.")
                            .required(true)
                            .kind(command::CommandOptionType::User)
                    })
                    .create_option(|option| {
                        option
                            .name("ign")
                            .description("The Minecraft in-game name of the person that should be screenshared.")
                            .required(true)
                            .kind(command::CommandOptionType::String)
                    })
            }).await?;
        Ok(())
    }

    fn initial_response(
        &self,
        _interaction_type: interaction::InteractionType,
    ) -> InitialInteractionResponse {
        InitialInteractionResponse::DeferEphemeralReply
    }

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        let in_question = UserId(command.get_str("user").unwrap().parse()?);
        let screenshare = crate::consts::DATABASE
            .fetch_screenshares_for(command.user.id.0)
            .await?;
        if let Some(screenshare) = screenshare {
            return Err(ErrorResponse::with_title(
                "One at a time please",
                format!(
                    "You already have a screenshare request open at <#{}>.",
                    screenshare.channel_id
                ),
            ))?;
        }

        let channel = create_screenshare_ticket(ctx, command.user.id, in_question)
            .await
            .map_err(|_| {
                ErrorResponse::message("Your screenshare channel couldn't be created...")
            })?;

        let ign = command.get_str("ign").unwrap();

        let message = channel
            .send_message(&ctx, |m| {
                *m = screenshare_message(command.user.id, in_question, ign);
                m
            })
            .await;

        if let Err(err) = message {
            let _ = channel.delete(&ctx).await.map_err(|err| {
                tracing::error!(
                    "Failed to delete screenshare channel after message failed: {}",
                    err
                )
            });
            return Err(Box::new(err));
        }

        let res = crate::consts::DATABASE
            .add_screenshare(channel.id.0, command.user.id.0, in_question.0)
            .await;
        if let Err(err) = res {
            let _ = channel
                .send_message(
                    &ctx.http,
                    |x| x.content("Due to an error occurring in the database, this ticket may not work as expected."),
                ).await.map_err(|err| tracing::error!("Failed to send warning in screenshare ticket after error in database: {}", err));
            tracing::error!("Failed to add screenshare to database: {}", err)
        }

        tokio::spawn(ticket_timeout(
            ctx.clone(),
            in_question,
            command.user.id,
            channel.id,
        ));

        let mut resp = CreateInteractionResponseData::default();
        resp.content(format!("Ticket created at {}.", channel.mention()));
        Ok(Some(resp))
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

async fn ticket_timeout(ctx: Context, in_question: UserId, closer: UserId, channel: ChannelId) {
    sleep(Duration::from_secs(15 * 60)).await;
    let not_frozen = crate::consts::DATABASE
        .fetch_freezes_for(in_question.0)
        .await
        .unwrap_or(None)
        .is_none();
    if not_frozen {
        let result = close::close_ticket(&ctx, closer, channel).await;
        if let Err(err) = result {
            tracing::error!("Failed to close ticket: {}", err)
        }
    }
}

async fn create_screenshare_ticket(
    ctx: &Context,
    creator: UserId,
    in_question: UserId,
) -> crate::Result<GuildChannel> {
    let channels = crate::CONFIG.guild.channels(&ctx.http).await?;
    let category = channels
        .iter()
        .find(|ch| {
            ch.1.kind == ChannelType::Category && ch.0 == &crate::CONFIG.screenshare_requests
        })
        .ok_or(serenity::Error::Other(
            "Screenshare category does not exist!",
        ))?;

    let count = sqlx::query!("SELECT COUNT(*) FROM screenshare")
        .fetch_one(&DATABASE.get())
        .await?
        .count;

    let guild_channel = crate::CONFIG
        .guild
        .create_channel(&ctx.http, |ch| {
            ch.name(format!("screenshare-{}", count.unwrap_or_default() + 1))
                .category(category.0)
                .kind(ChannelType::Text)
                .permissions(
                    // Iterator black magic
                    std::iter::repeat((*ALLOW_PERMS, *DENY_PERMS))
                        // Creator, Screensharers and in question
                        .zip([
                            PermissionOverwriteType::Member(creator),
                            PermissionOverwriteType::Member(in_question),
                            PermissionOverwriteType::Role(crate::CONFIG.ss_support),
                        ])
                        .map(|((allow, deny), kind)| PermissionOverwrite { allow, deny, kind })
                        .chain(std::iter::once(PermissionOverwrite {
                            allow: Permissions::empty(),
                            deny: *ALLOW_PERMS,
                            kind: PermissionOverwriteType::Role(crate::CONFIG.guild.0.into()),
                        })),
                )
        })
        .await?;
    Ok(guild_channel)
}

fn screenshare_message<'a>(creator: UserId, in_question: UserId, ign: String) -> CreateMessage<'a> {
    let mut msg = CreateMessage::default();
    msg.content(format!(
        "\
                {} \n\
                {} Please explain why you suspect {} of cheating and send us the screenshots of \
                you telling them not to log as well as any other info you can provide.\
            ",
        crate::consts::CONFIG.ss_support.mention(),
        creator.mention(),
        in_question.mention()
    ))
    .embed(|embed| {
        embed
            .title("Screenshare Request")
            .color(0xf03291)
            .description(format!(
                "\
                        If {} is not frozen by us within the next 15 minutes, \
                        this will automatically get deleted and they are safe to logout.
                    ",
                in_question.mention()
            ))
            .field("Minecraft Account", format!("```{}```", ign), false)
    })
    .components(|components| {
        components.create_action_row(|row| {
            row.create_button(|button| {
                button
                    .label("Freeze")
                    .custom_id(format!("freeze:{}", in_question))
                    .style(component::ButtonStyle::Primary)
                    .emoji(ReactionType::try_from(crate::CONFIG.freeze_emoji.clone()).unwrap())
            })
            .create_button(|button| {
                button
                    .label("Close")
                    .custom_id("close")
                    .style(component::ButtonStyle::Danger)
            })
        })
    });
    msg
}
