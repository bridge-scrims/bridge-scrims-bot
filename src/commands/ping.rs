use serenity::{
    async_trait,
    model::{
        id::RoleId,
        prelude::{
            application_command::{
                ApplicationCommandInteraction, ApplicationCommandOptionType,
                ApplicationCommandPermissionType,
            },
            InteractionApplicationCommandCallbackDataFlags,
        },
    },
    prelude::Context,
};
use std::time::Duration;

use crate::{
    commands::{Command, Cooldowns},
    consts::CONFIG,
    interact_opts::InteractOpts,
};

pub struct Ping {
    cooldowns: Cooldowns,
}

#[async_trait]
impl Command for Ping {
    fn name(&self) -> String {
        "ping".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd = CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name())
                    .description("Ping a desired role uppon request")
                    .default_permission(false);
                for option in &CONFIG.pings {
                    cmd.create_option(|x| {
                        x.kind(ApplicationCommandOptionType::SubCommand)
                            .name(&option.name)
                            .description(format!("Ping a role for the {} category", option.name))
                            .create_sub_option(|m| {
                                m.name("role")
                                    .kind(ApplicationCommandOptionType::String)
                                    .description("The role you would like to mention")
                                    .required(true);
                                for (name, role) in &option.options {
                                    m.add_string_choice(name, role.0);
                                }
                                m
                            })
                            .create_sub_option(|x| {
                                x.name("text")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(false)
                                    .description(
                                        "An optional additional text to put in the message",
                                    )
                            })
                    });
                }
                cmd
            })
            .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx.http, cmd.id, |perms| {
                for option in &CONFIG.pings {
                    perms.create_permission(|p| {
                        p.kind(ApplicationCommandPermissionType::Role)
                            .id(option.required_role.0)
                            .permission(true)
                    });
                }
                perms
            })
            .await?;
        Ok(())
    }
    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        // get the sub command
        let cmd = &command.data.options[0];

        let u = command.member.as_ref().unwrap();
        for opt in &CONFIG.pings {
            if opt.name == cmd.name && !u.roles.contains(&opt.required_role) {
                command
                    .create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| {
                            d.content("You are missing the required role to do this.")
                                .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                        })
                    })
                    .await?;
                return Ok(());
            }
        }
        let role = RoleId(cmd.get_str("role").unwrap().parse().unwrap());
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
                        .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                    })
                })
                .await?;
            return Ok(());
        }
        self.cooldowns
            .add_global_cooldown_key(cid.clone(), Duration::from_secs(60))
            .await;
        self.cooldowns
            .add_user_cooldown_key(cid.clone(), Duration::from_secs(60 * 5), command.user.id)
            .await;
        let text = cmd.get_str("text").unwrap_or_else(|| "".to_string());
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.content(format!("<@&{}> {}", role.0, text))
                        .allowed_mentions(|m| m.roles([role]))
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
