use serenity::{
    async_trait, builder::CreateInteractionResponseData, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::prelude::*,
};

use crate::consts::{CONFIG, DATABASE};
use bridge_scrims::interaction::*;

pub struct Unfreeze;

#[async_trait]
impl InteractionHandler for Unfreeze {
    fn name(&self) -> String {
        String::from("unfreeze")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec![crate::CONFIG.ss_support])
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |command| {
                command
                    .name(self.name())
                    .description("Unfreezes a user")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|opt| {
                        opt.name("player")
                            .description("The player to unfreeze")
                            .kind(command::CommandOptionType::User)
                            .required(true)
                    })
            })
            .await?;
        Ok(())
    }

    fn initial_response(
        &self,
        _interaction_type: interaction::InteractionType,
    ) -> InitialInteractionResponse {
        InitialInteractionResponse::DeferReply
    }

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        let user = UserId(command.get_str("player").unwrap().parse()?);
        let res = unfreeze_user(ctx, user).await?;
        add_screensharer(command.user.id).await?;
        Ok(res)
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

pub async fn add_screensharer(sser: UserId) -> crate::Result<()> {
    match DATABASE.get_screensharer(sser.0).await? {
        None => {
            DATABASE
                .set_screensharer(crate::db::Screensharer {
                    user_id: sser.0,
                    freezes: 1,
                })
                .await?
        }
        Some(mut sser) => {
            sser.freezes += 1;
            DATABASE.set_screensharer(sser).await?
        }
    }
    Ok(())
}

pub async fn unfreeze_user<'a>(ctx: &Context, user: UserId) -> InteractionResult<'a> {
    let freeze = DATABASE
        .fetch_freezes_for(user.0)
        .await?
        .ok_or_else(|| ErrorResponse::message(format!("{} is not frozen.", user.mention())))?;

    let mut roles: Vec<RoleId> = freeze.roles.into_iter().map(RoleId).collect();
    if !roles.contains(&CONFIG.member_role) {
        roles.push(CONFIG.member_role)
    }

    let member = CONFIG.guild.member(&ctx, user).await.map_err(|_| {
        ErrorResponse::message(format!(
            "{} can't be unfrozen since they left the server.",
            user.mention()
        ))
    })?;

    let keep_roles = member
        .roles(ctx)
        .unwrap_or_default()
        .iter()
        .filter(|r| r.managed)
        .map(|r| r.id)
        .collect::<Vec<_>>();

    let new_roles = keep_roles.iter().chain(roles.iter().filter(|r| {
        ctx.cache
            .guild_roles(CONFIG.guild.0)
            .unwrap()
            .contains_key(r)
    }));

    member.edit(&ctx, |m| m.roles(new_roles)).await?;

    // Member already has their roles back so it doesn't really matter if this fails
    sqlx::query!("DELETE FROM freezes WHERE user_id = $1", user.0 as i64)
        .execute(&DATABASE.get())
        .await?;

    let mut response = CreateInteractionResponseData::default();
    response.content(format!(
        "{} {}, you are now unfrozen",
        CONFIG.unfreeze_emoji,
        user.mention()
    ));
    Ok(Some(response))
}
