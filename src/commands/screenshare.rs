use futures::StreamExt;
use std::{fmt::Display, time::Duration};

use serenity::model::interactions::InteractionResponseType;
use serenity::{
    async_trait,
    builder::CreateMessage,
    client::Context,
    model::{
        channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType, ReactionType},
        id::UserId,
        interactions::{
            application_command::{
                ApplicationCommandInteraction as ACI, ApplicationCommandOptionType,
            },
            message_component::ButtonStyle,
            InteractionApplicationCommandCallbackDataFlags,
        },
        Permissions,
    },
};

use bridge_scrims::{
    hypixel::{Player, PlayerDataRequest},
    interact_opts::InteractOpts,
};

use super::{close, freeze::Freeze};
use crate::commands::{Button, Command};

lazy_static::lazy_static! {
    // allow:
    // VIEW_CHANNEL, SEND_MESSAGES, ATTACH_FILES, EMBED_LINKS
    pub static ref ALLOW_PERMS: Permissions = Permissions::from_bits(52224).unwrap();
    // deny: MENTION_EVERYONE
    pub static ref DENY_PERMS: Permissions = Permissions::from_bits(131072).unwrap();

}

#[derive(Clone, Copy)]
pub enum Operation {
    Close,
    Freeze,
}

#[derive(Debug)]
pub struct OperationDoesNotExist;

impl std::error::Error for OperationDoesNotExist {}
impl Display for OperationDoesNotExist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Operation does not exist")
    }
}

impl TryFrom<&'_ str> for Operation {
    type Error = OperationDoesNotExist;

    fn try_from(value: &'_ str) -> Result<Self, Self::Error> {
        match value {
            "close" => Ok(Self::Close),
            "freeze" => Ok(Self::Freeze),
            _ => Err(OperationDoesNotExist),
        }
    }
}

pub struct Screenshare;

#[async_trait]
impl Command for Screenshare {
    fn name(&self) -> String {
        String::from("screenshare")
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::CONFIG
            .guild
            .create_application_command(&ctx.http, |command| {
                command
                    .name(self.name())
                    .description("Creates a screenshare ticket")
                    .create_option(|option| {
                        option
                            .name("player")
                            .description("The person to request a screenshare to.")
                            .required(true)
                            .kind(ApplicationCommandOptionType::User)
                    })
                    .create_option(|option| {
                        option
                            .name("ign")
                            .description("The Minecraft ingame name of the person that you want to be screenshared.")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String)
                    })
            })
            .await?;
        Ok(())
    }
    async fn run(&self, ctx: &Context, command: &ACI) -> crate::Result<()> {
        let in_question = UserId(command.get_str("player").unwrap().parse()?);
        let screenshare = crate::consts::DATABASE.fetch_screenshares_for(command.user.id.0);
        if let Some(screenshare) = screenshare {
            command
                .create_interaction_response(&ctx.http, |msg| {
                    msg.interaction_response_data(|data| {
                        data.content(format!(
                            "You already have an active screenshare in <#${}>",
                            screenshare.id
                        ))
                        .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                    })
                })
                .await?;
            return Ok(());
        }

        let result: Result<_, serenity::Error> = {
            let channels = crate::CONFIG.guild.channels(&ctx.http).await?;
            let category = channels
                .iter()
                .find(|ch| {
                    ch.1.kind == ChannelType::Category
                        && ch.0 == &crate::CONFIG.screenshare_requests
                })
                .ok_or(serenity::Error::Other("Channel does not exist."))?;

            let mut count: Option<i64> = None;
            crate::consts::DATABASE.count_rows("Screenshares", "", |val| {
                if let sqlite::Value::Integer(co) = val[0] {
                    count = Some(co);
                }
            });

            let new_channel = crate::CONFIG
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
                                    PermissionOverwriteType::Member(command.user.id),
                                    PermissionOverwriteType::Member(in_question),
                                    PermissionOverwriteType::Role(crate::CONFIG.ss_support),
                                ])
                                .map(|((allow, deny), kind)| PermissionOverwrite {
                                    allow,
                                    deny,
                                    kind,
                                })
                                .chain(std::iter::once(PermissionOverwrite {
                                    allow: Permissions::empty(),
                                    deny: Permissions::READ_MESSAGES,
                                    kind: PermissionOverwriteType::Role(
                                        crate::CONFIG.guild.0.into(),
                                    ),
                                })),
                        )
                })
                .await?;
            Ok(new_channel)
        };

        if let Ok(channel) = result {
            let mut message = CreateMessage::default();

            let name = command.get_str("ign").unwrap();
            let player = Player::fetch_from_username(name.clone()).await?;
            let playerstats = PlayerDataRequest(crate::CONFIG.hypixel_token.clone(), player)
                .send()
                .await
                .unwrap_or_default();

            let db_result = crate::consts::DATABASE.add_screenshare(
                channel.id.0,
                command.user.id.0,
                in_question.0,
            );
            if db_result.is_err() {
                channel
                    .send_message(
                        &ctx.http,
                       |x| x.content("An error occured in the database. The ticket may not work as expected."),
                    )
                    .await?;
            }

            message.content(format!(
                "<@&{}>
<@{}> Please explain how <@{}> is cheating and screenshots of you telling them
not to log aswell as any other info.
",
                crate::consts::CONFIG.ss_support,
                command.user.id.0,
                in_question
            ));

            message.embed(|embed| {
                embed
                    .title("Screenshare Request")
                    .description(
                        "- Why did you request a screenshare on this member?
- Please provide evidence of you telling him not to log.
- Anything else?

**NOTE**: If you do not get frozen within 15 minutes you may logout.
",
                    )
                    .field("Ign", name, false)
                    .field(
                        "Last login time",
                        playerstats.last_login.unwrap_or_default(),
                        false,
                    )
                    .field(
                        "Last logout time",
                        playerstats.last_logout.unwrap_or_default(),
                        false,
                    )
            });
            message.components(|components| {
                components.create_action_row(|row| {
                    row.create_button(|button| {
                        button
                            .label("Freeze")
                            .style(ButtonStyle::Primary)
                            .emoji(ReactionType::Custom {
                                animated: false,
                                id: crate::CONFIG.freeze_emoji,
                                name: None,
                            })
                            .custom_id(format!("freeze:{}", in_question))
                    })
                    .create_button(|button| {
                        button
                            .label("Close")
                            .style(ButtonStyle::Danger)
                            .emoji(ReactionType::Unicode(From::from("⛔")))
                            .custom_id(format!("close:{}", channel.id))
                    })
                })
            });
            let mut m = channel.send_message(&ctx.http, |_| &mut message).await?;
            command
                .create_interaction_response(&ctx.http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content(format!("Ticket created in {}", channel))
                            .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                    })
                })
                .await?;

            let mut reactions = m
                .await_component_interactions(&ctx)
                .timeout(Duration::from_secs(60 * 15))
                .await;

            while let Some(reaction) = reactions.next().await {
                if reaction.user.id == in_question || reaction.user.id == command.user.id {
                    let _ = reaction
                        .create_interaction_response(&ctx, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|r| {
                                    r.flags(
                                        InteractionApplicationCommandCallbackDataFlags::EPHEMERAL,
                                    )
                                    .content("You do not have permission to do that")
                                })
                        })
                        .await;
                    continue;
                }
                m.edit(&ctx, |m| {
                    m.components(|comp| comp.set_action_rows(Default::default()))
                })
                .await?;

                let mut chunks = reaction.data.custom_id.split(':');
                let operation = chunks.next().unwrap_or_default();
                let operation = Operation::try_from(operation)?;
                let operation: Box<dyn Button> = match operation {
                    Operation::Close => close::Close::new(),
                    Operation::Freeze => Freeze::new(),
                };

                operation
                    .click(ctx, &*reaction)
                    .await
                    .map_err(|x| format!("While handling button: {}", x))?;
                return Ok(());
            }
            // This is so you also can use /freeze
            if crate::consts::DATABASE
                .fetch_freezes_for(in_question.0)
                .is_none()
            {
                close::close_ticket(ctx, command.user.id, channel.id).await?;
            }
        } else {
            command
                .create_interaction_response(&ctx.http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content(format!(
                            "Could not create your ticket: {}",
                            result.as_ref().unwrap_err()
                        ))
                        .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                    })
                })
                .await?;
        }
        Ok(())
    }
    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
