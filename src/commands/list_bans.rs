use std::fmt::Write;

use serenity::model::application::command::CommandOptionType;
use serenity::model::Permissions;
use serenity::{
    async_trait, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
};

use bridge_scrims::interact_opts::InteractOpts;

use super::Command;

pub struct ListBans;

#[async_trait]
impl Command for ListBans {
    fn name(&self) -> String {
        String::from("list_bans")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name())
                    .description("List all of the (scrim)bans")
                    .create_option(|opt| {
                        opt.name("type")
                            .description("Wether you want to list scrimbans or server bans")
                            .required(true)
                            .kind(CommandOptionType::String)
                            .add_string_choice("Scrim", "sc")
                            .add_string_choice("Server", "sv")
                    })
                    .default_member_permissions(Permissions::empty())
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
        let mut desc = match operation.as_str() {
            "sv" => {
                let bans = crate::consts::DATABASE.fetch_unbans();
                let mut result = vec![String::new()];
                for ban in bans {
                    if result[result.len() - 1].len() > 1950 {
                        result.push(String::new())
                    }
                    let t = result.len() - 1;
                    writeln!(
                        result[t],
                        "- <@!{}>: banned until <t:{}:R>",
                        ban.id,
                        ban.date.unix_timestamp()
                    )?;
                }
                result
            }
            "sc" => {
                let bans = crate::consts::DATABASE.fetch_scrim_unbans();
                let mut result = vec![String::new()];
                for ban in bans {
                    if result[result.len() - 1].len() > 1950 {
                        result.push(String::new())
                    }
                    let t = result.len() - 1;
                    writeln!(
                        result[t],
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
                    data.embed(|embed| {
                        embed
                            .title(format!(
                                "{} Bans",
                                if operation.as_str() == "sv" {
                                    "Server"
                                } else {
                                    "Scrim"
                                }
                            ))
                            .description(desc.pop().unwrap())
                    })
                })
            })
            .await?;
        for d in desc {
            command
                .create_followup_message(&ctx.http, |resp| resp.embed(|embed| embed.description(d)))
                .await?;
        }
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
