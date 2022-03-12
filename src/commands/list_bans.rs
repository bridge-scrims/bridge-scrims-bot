use bridge_scrims::interact_opts::InteractOpts;
use serenity::{
    async_trait,
    client::Context,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandOptionType,
        ApplicationCommandPermissionType,
    },
};
use std::fmt::Write;

use crate::consts::CONFIG;

use super::Command;

pub struct ListBans;

#[async_trait]
impl Command for ListBans {
    fn name(&self) -> String {
        String::from("list_bans")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = crate::CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name())
                    .description("List all of the (scrim)bans")
                    .create_option(|opt| {
                        opt.name("type")
                            .description("Wether you want to list scrimbans or server bans")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String)
                            .add_string_choice("Scrim", "sc")
                            .add_string_choice("Server", "sv")
                    })
                    .default_permission(false)
            })
            .await?;
        crate::CONFIG
            .guild
            .create_application_command_permission(&ctx.http, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.staff.0)
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
        let operation = command.get_str("type").unwrap();
        let desc = match operation.as_str() {
            "sv" => {
                let bans = crate::consts::DATABASE.fetch_unbans();
                let mut result = String::new();
                for ban in bans {
                    writeln!(
                        result,
                        "- <@!{}>: banned until <t:{}:R>",
                        ban.id,
                        ban.date.unix_timestamp()
                    )?;
                }
                result
            }
            "sc" => {
                let bans = crate::consts::DATABASE.fetch_scrim_unbans();
                let mut result = String::new();
                for ban in bans {
                    writeln!(
                        result,
                        "- <@!{}>: banned until <t:{}:R>",
                        ban.id,
                        ban.date.unix_timestamp()
                    )?;
                }
                result
            }
            _ => {
                return Ok(());
            }
        };
        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.interaction_response_data(|data| {
                    data.create_embed(|embed| {
                        embed
                            .title(format!(
                                "{} Bans",
                                if operation.as_str() == "sv" {
                                    "Server"
                                } else {
                                    "Scrim"
                                }
                            ))
                            .description(desc)
                    })
                })
            })
            .await?;
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
