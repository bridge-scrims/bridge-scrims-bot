use std::time::Duration;

use serenity::model::application::command::CommandOptionType;
use serenity::model::Permissions;
use serenity::{
    async_trait,
    model::{
        application::interaction::{
            application_command::ApplicationCommandInteraction, MessageFlags,
        },
        id::RoleId,
    },
    prelude::Context,
};

use bridge_scrims::{cooldown::Cooldowns, interact_opts::InteractOpts};

use crate::{commands::Command, consts::CONFIG};

pub struct Ping {
    cooldowns: Cooldowns,
}

#[async_trait]
impl Command for Ping {
    fn name(&self) -> String {
        "ping".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        for option in &CONFIG.pings {
            CONFIG
                .guild
                .create_application_command(&ctx.http, |cmd| {
                    cmd.name(option.name.clone())
                        .description("Ping a desired role uppon request")
                        .default_member_permissions(Permissions::empty())
                        .create_option(|m| {
                            m.name("role")
                                .kind(CommandOptionType::String)
                                .description("The role you would like to mention")
                                .required(true);
                            for (name, role) in &option.options {
                                m.add_string_choice(name, role.0);
                            }
                            m
                        })
                        .create_option(|x| {
                            x.name("text")
                                .kind(CommandOptionType::String)
                                .required(false)
                                .description("An optional additional text to put in the message")
                        });
                    cmd
                })
                .await?;
        }
        Ok(())
    }
    fn is_command(&self, name: String) -> bool {
        CONFIG.pings.iter().any(|opt| opt.name == name)
    }
    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let role = RoleId(command.get_str("role").unwrap().parse().unwrap());
        if let Some(opt) = CONFIG
            .pings
            .iter()
            .find(|opt| opt.name == command.data.name)
        {
            if let Some(channels) = &opt.allowed_channels {
                let cat = command
                    .channel_id
                    .to_channel(&ctx)
                    .await?
                    .guild()
                    .unwrap()
                    .parent_id;
                if !channels
                    .iter()
                    .any(|c| c == &command.channel_id || Some(*c) == cat)
                {
                    command
                        .create_interaction_response(&ctx.http, |r| {
                            r.interaction_response_data(|d| {
                                d.content("This command is disabled in this channel.")
                                    .flags(MessageFlags::EPHEMERAL)
                            })
                        })
                        .await?;
                    return Ok(());
                }
            }
        }
        let cid = format!("{}", role.0);
        let cooldown = self
            .cooldowns
            .check_cooldown_key(command.user.id, cid.clone())
            .await;
        if let Some(t) = cooldown {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content(format!(
                            "You are on a cooldown. Please wait {:.2} seconds.",
                            t.as_secs_f32()
                        ))
                        .flags(MessageFlags::EPHEMERAL)
                    })
                })
                .await?;
            return Ok(());
        }
        self.cooldowns
            .add_global_cooldown_key(cid.clone(), Duration::from_secs(20))
            .await;
        self.cooldowns
            .add_user_cooldown_key(cid.clone(), Duration::from_secs(35), command.user.id)
            .await;
        let text = command.get_str("text").unwrap_or_else(|| "".to_string());
        command
            .channel_id
            .send_message(&ctx.http, |r| {
                r.content(format!("<@!{}>: <@&{}> {}", command.user.id, role.0, text))
                    .allowed_mentions(|m| m.roles(vec![role]))
            })
            .await?;
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.content("Ping sent!").flags(MessageFlags::EPHEMERAL)
                })
            })
            .await?;
        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Ping {
            cooldowns: Cooldowns::new(),
        })
    }
}
