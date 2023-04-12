use time::OffsetDateTime;

use serenity::{
    async_trait,
    client::Context,
    builder::CreateInteractionResponseData,

    model::prelude::*,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction,
        message_component::MessageComponentInteraction
    }
};

use bridge_scrims::interaction::*;

pub struct Freeze;

#[async_trait]
impl InteractionHandler for Freeze {
    
    fn name(&self) -> String {
        String::from("freeze")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec!(crate::CONFIG.ss_support))
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::CONFIG
            .guild
            .create_application_command(&ctx, |cmd| {
                cmd.name(self.name())
                    .description("Freezes a user")
                    .create_option(|opt| {
                        opt.name("player")
                            .kind(command::CommandOptionType::User)
                            .required(true)
                            .description("The player to be frozen")
                    })
                    .default_member_permissions(Permissions::empty())
            })
            .await?;
        Ok(())
    }

    fn initial_response(&self, _interaction_type: interaction::InteractionType) -> InitialInteractionResponse {
        InitialInteractionResponse::DeferEphemeralReply
    }

    async fn handle_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult
    {
        let user = UserId(command.get_str("player").unwrap().parse()?);
        freeze_user(ctx, user, command.user.id).await
    }

    async fn handle_component(&self, ctx: &Context, command: &MessageComponentInteraction, args: &[&str]) -> InteractionResult
    {
        let user = UserId(args.first().unwrap().parse()?);
        freeze_user(ctx, user, command.user.id).await
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

async fn freeze_user<'a>(ctx: &Context, target: UserId, executor: UserId) -> InteractionResult<'a>
{
    let executor = crate::CONFIG.guild.member(&ctx, executor).await?;
    let executor_roles = executor.roles(&ctx.cache).unwrap_or_default();
    let executors_highest = executor_roles.iter().map(|r| r.position).max().unwrap_or_default();
    
    let member = crate::CONFIG.guild.member(&ctx, target).await?;
    let targets_roles = member.roles(&ctx.cache).unwrap_or_default();
    let targets_role_ids = targets_roles.iter().map(|r| r.id).collect::<Vec<_>>();
    let targets_highest = targets_roles.iter().map(|r| r.position).max().unwrap_or_default();

    if executors_highest <= targets_highest {
        return Err(
            ErrorResponse::with_title(
                "Insufficient Permissions", 
                format!("You are missing the required permissions to freeze {}!", target.mention())
            )
        )?;
    }

    let is_frozen = crate::consts::DATABASE.fetch_freezes_for(target.0).is_some();
    if is_frozen {
        return Err(ErrorResponse::message(format!("{} is already frozen.", target.mention())))?;
    }

    let is_banned = crate::consts::DATABASE.fetch_scrim_unbans().iter().any(|x| !x.is_expired() && x.id == target.0);
    if is_banned {
        return Err(ErrorResponse::message(format!("{} is already banned.", target.mention())))?;
    }

    let mut new_roles = targets_roles.iter().filter(|r| r.managed).map(|r| r.id).collect::<Vec<_>>();
    new_roles.push(crate::CONFIG.frozen);
    let removed_roles = targets_role_ids.clone().into_iter().filter(|r| !new_roles.contains(r)).collect::<Vec<_>>();
    
    member.edit(&ctx, |m| m.roles(new_roles)).await?;
    let res = crate::consts::DATABASE.add_freeze(
        target.0,
        removed_roles.into(),
        OffsetDateTime::now_utc(),
    );
    if let Err(err) = res {
        // This is already a fail-safe so errors here are ignored
        let _ = member.edit(&ctx, |m| m.roles(targets_role_ids.clone())).await
            .map_err(|_| println!("Failed to give {} back their roles ({:?}) after freeze failed!", target, targets_role_ids));
        return Err(Box::new(err));
    }

    // This is ignored since at this point the user has already been frozen, thus it's too late to abort
    let _ = crate::CONFIG
        .frozen_chat
        .send_message(&ctx, |msg| {
            msg.content(format!(
                "\
                    Hello {}, would you like to admit to cheating for a shortened ban or would \
                    you like us to search through your computer for cheats? You have 5 minutes \
                    to either admit in this channel or join {} and follow the instructions below.\
                    \n \n\
                    **Download AnyDesk from here:** \n\
                    Windows: https://download.anydesk.com/AnyDesk.exe \n\
                    Mac: https://download.anydesk.com/anydesk.dmg \n\
                    Once it's downloaded, run it and **we will need you to send your 9 digit address code** in this channel.\
                    \n \n\
                    While screensharing we will download three screenshare tools and require admin \
                    control. Whilst the screenshare is happening **we will need you to __not__ touch your \
                    mouse or keyboard unless instructed** to do anything. Failure to comply with what \
                    we say will result in a ban. \
                    \n \n\
                    Our screensharers **will __not__ be going through personal files or attempting to harm your computer**. \
                    We will only be checking for cheats by inspecting your mouse & keyboard software, recycle bin, deleted files \
                    and applications ran on this instance of your pc, as well as by running pre-bundled, trusted screenshare tools.\
                ", target.mention(), crate::CONFIG.hello_cheaters.mention()
            )).flags(MessageFlags::SUPPRESS_EMBEDS)
        }).await
            .map_err(|e| tracing::error!("Failed to send freeze message: {}", e));
    
    let mut response = CreateInteractionResponseData::default();
    response.content(format!("Successfully froze {}.", target.mention()));
    Ok(Some(response))
}