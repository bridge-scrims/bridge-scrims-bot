use std::time::Duration;

use serenity::model::application::command::CommandOptionType;
use serenity::{
    async_trait,
    client::Context,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction as ACI, MessageFlags,
    },
};

use bridge_scrims::{
    cooldown::Cooldowns,
    hypixel::{Player, PlayerDataRequest},
    interact_opts::InteractOpts,
};

use crate::commands::Command;

pub struct LogTime {
    cooldowns: Cooldowns,
}

#[async_trait]
impl Command for LogTime {
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
                            .description("The Minecraft ingame name of the person that you want to fetch the log time of.")
                            .required(true)
                            .kind(CommandOptionType::String)
                    })
            })
            .await?;
        Ok(())
    }
    async fn run(&self, ctx: &Context, command: &ACI) -> crate::Result<()> {
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
            return Ok(());
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
            return Ok(());
        }
        match PlayerDataRequest(crate::CONFIG.hypixel_token.clone(), player.unwrap())
            .send()
            .await
        {
            Ok(playerstats) => {
                command
                    .create_interaction_response(&ctx.http, |message| {
                        message.interaction_response_data(|d| {
                            d.embed(|em| {
                                em.title(format!("Log Time information for {}", name))
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
                                    .description("The hypixel api has yeileded an error!")
                            })
                            .flags(MessageFlags::EPHEMERAL)
                        })
                    })
                    .await?;
            }
        }
        Ok(())
    }
    fn new() -> Box<LogTime> {
        Box::new(LogTime {
            cooldowns: Cooldowns::new(),
        })
    }
}
