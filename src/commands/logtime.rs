use std::time::Duration;

use serenity::{
    async_trait, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::application::interaction::MessageFlags, model::prelude::*,
};

use bridge_scrims::{
    cooldown::Cooldowns,
    hypixel::{Player, PlayerDataRequest},
    interaction::*,
};

pub struct LogTime {
    cooldowns: Cooldowns,
}

#[async_trait]
impl InteractionHandler for LogTime {
    fn name(&self) -> String {
        "logtime".to_string()
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
                            .name("ign")
                            .description("The Minecraft in-game name of the person that you want to fetch the log time of.")
                            .required(true)
                            .kind(command::CommandOptionType::String)
                    })
            })
            .await?;
        Ok(())
    }

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        if let Some(t) = self.cooldowns.check_cooldown(command.user.id).await {
            command
                .create_interaction_response(&ctx.http, |m| {
                    m.interaction_response_data(|d| {
                        d.flags(MessageFlags::EPHEMERAL).content(format!(
                            "You are currently on a cooldown for {:.2} seconds.",
                            t.as_secs_f32()
                        ))
                    })
                })
                .await?;
            return Ok(None);
        }
        self.cooldowns
            .add_global_cooldown(Duration::from_secs(5))
            .await;
        self.cooldowns
            .add_user_cooldown(Duration::from_secs(15), command.user.id)
            .await;
        let name = command.get_str("ign").unwrap();
        let player = Player::fetch_from_username(name.clone()).await;
        if let Err(err) = player {
            tracing::info!("Error in logtime: {}", err);
            command
                .create_interaction_response(&ctx.http, |message| {
                    message.interaction_response_data(|d| {
                        d.embed(|em| {
                            em.title("Invalid User!")
                                .description(format!("The player `{}` could not be found", name))
                        })
                        .flags(MessageFlags::EPHEMERAL)
                    })
                })
                .await?;
            return Ok(None);
        }
        match PlayerDataRequest(crate::SECRETS.hypixel_token.clone(), player.unwrap())
            .send()
            .await
        {
            Ok(player_stats) => {
                command
                    .create_interaction_response(&ctx.http, |message| {
                        message.interaction_response_data(|d| {
                            d.embed(|em| {
                                em.title(format!("Log Time information for {}", name))
                                    .field(
                                        "Last login time",
                                        player_stats.last_login.unwrap_or_default(),
                                        false,
                                    )
                                    .field(
                                        "Last logout time",
                                        player_stats.last_logout.unwrap_or_default(),
                                        false,
                                    )
                            })
                        })
                    })
                    .await?;
            }
            Err(err) => {
                tracing::info!("Error in logtime: {}", err);
                command
                    .create_interaction_response(&ctx.http, |message| {
                        message.interaction_response_data(|d| {
                            d.embed(|em| {
                                em.title("API Error!")
                                    .description("The Hypixel api has yielded an error!")
                            })
                            .flags(MessageFlags::EPHEMERAL)
                        })
                    })
                    .await?;
            }
        }
        Ok(None)
    }

    fn new() -> Box<LogTime> {
        Box::new(LogTime {
            cooldowns: Cooldowns::new(),
        })
    }
}
