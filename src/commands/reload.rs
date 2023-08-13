use std::collections::HashSet;

use serenity::{
    async_trait,
    builder::{CreateAutocompleteResponse, CreateInteractionResponseData},
    client::Context,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction, autocomplete::AutocompleteInteraction,
    },
    model::prelude::*,
};

use crate::{consts::CONFIG, handler::register_commands};
use bridge_scrims::interaction::*;

pub struct Reload;

#[async_trait]
impl InteractionHandler for Reload {
    fn name(&self) -> String {
        String::from("reload")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name())
                    .description("Reloads application commands.")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|opt| {
                        opt.name("command")
                            .description("Command to remove [Default: all]")
                            .kind(command::CommandOptionType::String)
                            .set_autocomplete(true)
                            .required(false)
                    })
            })
            .await?;

        Ok(())
    }

    async fn handle_autocomplete(
        &self,
        ctx: &Context,
        interaction: &AutocompleteInteraction,
        resp: &mut CreateAutocompleteResponse,
    ) -> AutocompleteResult {
        let focused = interaction.get_focused().unwrap();
        let selected = focused
            .value
            .as_ref()
            .map_or("", |v| v.as_str().unwrap_or(""));

        if focused.name == "command" {
            let commands = CONFIG.guild.get_application_commands(ctx).await?;

            crate::handler::HANDLERS
                .iter()
                .map(|c| c.name())
                .chain(commands.into_iter().map(|c| c.name))
                .collect::<HashSet<_>>()
                .into_iter()
                .filter(|name| name.contains(selected))
                .take(25)
                .for_each(|name| {
                    resp.add_string_choice(&name, &name);
                });

            return Ok(true);
        }

        Ok(false)
    }

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.kind(interaction::InteractionResponseType::DeferredChannelMessageWithSource)
                    .interaction_response_data(|d| d.flags(interaction::MessageFlags::EPHEMERAL))
            })
            .await?;

        let mut commands = CONFIG.guild.get_application_commands(&ctx.http).await?;

        if let Some(cmd) = command.get_str("command") {
            commands.retain(|x| x.name == cmd);
        }

        for c in &commands {
            CONFIG
                .guild
                .delete_application_command(&ctx.http, c.id)
                .await?;
        }

        let mut response = CreateInteractionResponseData::default();
        response.content(match commands.len() {
            0 => "Removed no commands.".to_string(),
            1 => format!(
                "Removed command `/{}`\nAdding command back now...",
                commands[0].name
            ),
            _ => "Removed all commands.\nAdding commands back now...".to_string(),
        });
        command.edit_response(&ctx.http, response).await?;

        let res = register_commands(ctx.clone()).await;
        command
            .create_followup_message(&ctx.http, |resp| {
                resp.content(match &res {
                    Ok(_) => "Successfully reloaded!".to_string(),
                    Err(err) => format!("Reloading failed: {}", err),
                })
                .flags(interaction::MessageFlags::EPHEMERAL)
            })
            .await?;

        if let Err(err) = res {
            tracing::error!("Reloading failed: {}", err);
        }

        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
