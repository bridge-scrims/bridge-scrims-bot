use crate::commands::Command;
use bridge_scrims::interact_opts::InteractOpts;
use chrono::{
    prelude::{DateTime, Utc},
    Duration,
};
use serenity::{
    async_trait,
    model::{
        id::UserId,
        interactions::{
            application_command::{
                ApplicationCommandInteraction, ApplicationCommandOptionType,
                ApplicationCommandPermissionType,
            },
            InteractionResponseType,
        },
    },
    prelude::Context,
    utils::Color,
};

use crate::consts::CONFIG;

pub struct Timeout {}

#[async_trait]
impl Command for Timeout {
    fn name(&self) -> String {
        "timeout".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd = CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Times a user out.")
                    .create_option(|o| {
                        o.name("user")
                            .description("The person you would like to time out.")
                            .required(true)
                            .kind(ApplicationCommandOptionType::User)
                    })
                    .create_option(|o| {
                        o.name("duration")
                            .description("The time you would like to time someone out for.")
                            .required(true)
                            .kind(ApplicationCommandOptionType::Integer)
                    })
                    .create_option(|o| {
                        o.name("type")
                            .description("The type of time you would like to use.")
                            .kind(ApplicationCommandOptionType::Integer)
                            .required(true)
                            .add_int_choice("Seconds", 1)
                            .add_int_choice("Minutes", 60)
                            .add_int_choice("Hours", 60 * 60)
                            .add_int_choice("Days", 60 * 60 * 24)
                    })
                    .default_permission(false)
            })
            .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx, cmd.id, |p| {
                for role in &[CONFIG.support, CONFIG.trial_support, CONFIG.staff] {
                    p.create_permission(|perm| {
                        perm.kind(ApplicationCommandPermissionType::Role)
                            .id(role.0)
                            .permission(true)
                    });
                }
                p
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

        let user = UserId(command.get_str("user").unwrap().parse()?)
            .to_user(&ctx.http)
            .await?;

        let duration = command.get_u64("duration").unwrap_or(3);
        let mult = command.get_i64("type").unwrap_or(60 * 60);

        let now: DateTime<Utc> = Utc::now();

        let duration = Duration::seconds(duration as i64 * mult);

        let end = now + duration;

        let mut member = ctx.http.get_member(CONFIG.guild.0, user.id.0).await?;
        let cmd_member = command.clone().member.unwrap();

        let roles = member.roles(&ctx.cache).await.unwrap_or_default();
        let cmd_roles = cmd_member.roles(&ctx.cache).await.unwrap_or_default();

        let top_role = roles.iter().max();
        let cmd_top_role = cmd_roles.iter().max();

        if top_role >= cmd_top_role || member.user.bot {
            command
                .edit_original_interaction_response(&ctx.http, |resp| {
                    resp.content(format!(
                        "You do not have permission to timeout {}",
                        user.tag()
                    ))
                })
                .await?;
            return Ok(());
        }

        let resp = member
            .disable_communication_until_datetime(&ctx.http, end)
            .await;

        command
            .edit_original_interaction_response(&ctx, |r| match resp {
                Ok(()) => r.create_embed(|e| {
                    e.title("User Timed out!")
                        .description(format!(
                            "The user {} has been timed out until <t:{}>.",
                            user.tag(),
                            end.timestamp()
                        ))
                        .color(Color::new(0x1abc9c))
                }),
                Err(err) => r.content(format!("Could not timeout {}: {}", user.tag(), err)),
            })
            .await?;

        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Timeout {})
    }
}
