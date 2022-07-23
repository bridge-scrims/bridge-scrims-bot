use serenity::model::application::command::{CommandOptionType, CommandPermissionType};
use serenity::model::Permissions;
use serenity::{
    async_trait,
    client::Context,
    model::{
        application::interaction::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction, MessageFlags,
        },
        id::{ChannelId, UserId},
    },
};
use time::OffsetDateTime;

use bridge_scrims::interact_opts::InteractOpts;

use super::{Button, Command};

#[non_exhaustive]
#[must_use = "statuses should be handled in the response to the user"]
enum Status {
    Success,
    Ignored,
}

pub struct Freeze;

#[async_trait]
impl Command for Freeze {
    fn name(&self) -> String {
        String::from("freeze")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = crate::CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name())
                    .description("Freezes a user")
                    .create_option(|opt| {
                        opt.name("player")
                            .kind(CommandOptionType::User)
                            .required(true)
                            .description("The player to be frozen")
                    })
                    .default_member_permissions(Permissions::empty())
            })
            .await?;

        crate::CONFIG
            .guild
            .create_application_command_permission(&ctx, command.id, |p| {
                p.create_permission(|perm| {
                    perm.kind(CommandPermissionType::Role)
                        .id(crate::CONFIG.ss_support.0)
                        .permission(true)
                })
            })
            .await?;

        Ok(())
    }

    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let user = UserId(command.get_str("player").unwrap().parse()?);
        let status = freeze_user(ctx, user, command.user.id, command.channel_id).await?;
        let tag = user.to_user(&ctx.http).await?.tag();
        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.interaction_response_data(|data| match status {
                    Status::Success => data.content(format!("Sucessfully frozen {}", tag)),
                    Status::Ignored => data
                        .content(format!("Ignored your request to freeze {}.", tag))
                        .flags(MessageFlags::EPHEMERAL),
                })
            })
            .await?;
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

#[async_trait]
impl Button for Freeze {
    async fn click(
        &self,
        ctx: &Context,
        command: &MessageComponentInteraction,
    ) -> crate::Result<()> {
        let user = UserId(
            command
                .data
                .custom_id
                .split(':')
                .nth(1)
                .unwrap_or_default()
                .parse()?,
        );
        if !command
            .user
            .has_role(&ctx.http, crate::CONFIG.guild, crate::CONFIG.ss_support)
            .await?
        {
            command
                .create_interaction_response(&ctx.http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content("You are not a screensharer!")
                            .flags(MessageFlags::EPHEMERAL)
                    })
                })
                .await?;
            return Ok(());
        }
        let status = freeze_user(ctx, user, command.user.id, command.channel_id).await?;
        let tag = user.to_user(&ctx.http).await?.tag();
        if let Status::Success = status {
            command
                .create_interaction_response(&ctx.http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content(format!("Successfully frozen {}", tag))
                            .flags(MessageFlags::EPHEMERAL)
                    })
                })
                .await?;
        }

        Ok(())
    }
}

async fn freeze_user(
    ctx: &Context,
    target: UserId,
    staff: UserId,
    channel: ChannelId,
) -> crate::Result<Status> {
    let emoji = crate::CONFIG
        .guild
        .emoji(&ctx.http, crate::CONFIG.unfreeze_emoji)
        .await?;
    let is_frozen = crate::consts::DATABASE
        .fetch_freezes_for(target.0)
        .is_some();
    if is_frozen {
        already_frozen(ctx, channel, target).await?;
        return Ok(Status::Ignored);
    }

    if crate::consts::DATABASE
        .fetch_scrim_unbans()
        .iter()
        .any(|x| x.id == target.0)
    {
        channel
            .send_message(&ctx.http, |msg| {
                msg.embed(|emb| {
                    emb.title("Already banned.")
                        .description(format!("<@!{}> is already banned.", target.0))
                })
            })
            .await?;
        return Ok(Status::Ignored);
    }

    let user = target.to_user(&ctx.http).await?;
    let mut member = crate::CONFIG.guild.member(&ctx.http, user.id).await?;

    let roles = member.roles(&ctx.cache).unwrap_or_default();
    let highest_role = roles
        .iter()
        .fold(0, |acc, x| if x.position > acc { x.position } else { acc });
    let staffroles = crate::CONFIG
        .guild
        .member(&ctx.http, staff)
        .await?
        .roles(&ctx.cache)
        .unwrap_or_default();
    let has_higher_role = staffroles
        .into_iter()
        .any(|srole| srole.position > highest_role);
    if !has_higher_role {
        channel
            .send_message(&ctx.http, |msg| {
                msg.embed(|emb| {
                    emb.title("Cannot freeze")
                        .description(format!(
                        "<@{}>, you are not allowed to freeze <@!{}> as they have a higher role than you.",
                        staff.0,
                        target.0
                    ))
                })
            })
            .await?;
        return Ok(Status::Ignored);
    }
    let mut removed_roles = Vec::new();

    // TODO: Make this into a function
    for role in roles.iter().filter(|x| !x.managed) {
        if member.remove_role(&ctx.http, role).await.is_ok() {
            removed_roles.push(role.id);
        }
    }

    member.add_role(&ctx.http, crate::CONFIG.frozen).await?;

    crate::consts::DATABASE.add_freeze(
        user.id.0,
        removed_roles.into(),
        OffsetDateTime::now_utc(),
    )?;
    crate::CONFIG
        .frozen_chat
        .send_message(&ctx.http, |msg| {
            msg.content(format!(
                "Hello <@{}>, would you like to admit to cheating for a shortened ban or would
you like me to search through your computer? If you want me to search you have 5
minutes to join the <#{}> and download the following applications.

Download Anydesk from here:
Windows: https://download.anydesk.com/AnyDesk.exe
Mac: https://download.anydesk.com/anydesk.dmg
Once you have downloaded AnyDesk I’ll need you to send me your 9 digit code in
the #frozen-chat channel


While screensharing I will download three screenshare tools and require admin
control. Whilst the screenshare is happening i will need you to not touch your
mouse or keyboard unless instructed to do anything. Failure to comply with what
I say will result in a ban.

As a screensharer I will not be going through personal files or attempting to harm your computer. I
will only be checking for cheats in the following areas: your mouse & keyboard software, run
screenshare tools, check recycle bin, revise deleted files, check for applications ran on this
instance of your pc, and revise your processes for cheats.",
                user.id,
                crate::CONFIG.hello_cheaters,
            ))
        })
        .await?;
    channel
        .send_message(&ctx.http, |msg| {
            msg.content(format!("{}: {} is now frozen by <@{}>", emoji, user, staff))
        })
        .await?;

    Ok(Status::Success)
}

async fn already_frozen(ctx: &Context, channel: ChannelId, user: UserId) -> crate::Result<()> {
    channel
        .send_message(&ctx.http, |msg| {
            msg.embed(|embed| {
                embed
                    .title("Already frozen")
                    .description(format!("<@!{}> is already frozen", user.0))
            })
        })
        .await?;
    Ok(())
}
