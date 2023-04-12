use serenity::{
    async_trait,
    client::Context,

    model::prelude::*,
    model::application::interaction::application_command::ApplicationCommandInteraction
};

use bridge_scrims::interaction::*;
use crate::consts::CONFIG;

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
                        let opt = opt
                            .name("command")
                            .description("Which command to remove. Default: all")
                            .kind(command::CommandOptionType::String)
                            .required(false);
                        for command in crate::handler::HANDLERS.iter() {
                            let name = command.name();
                            opt.add_string_choice(&name, &name);
                        }
                        opt
                    })
            })
            .await?;

        Ok(())
    }

    async fn handle_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult
    {
        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.kind(interaction::InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        let mut commands = CONFIG.guild.get_application_commands(&ctx.http).await?;

        if let Some(cmd) = command.get_str("command") {
            commands.retain(|x| x.name == cmd);
        }

        for c in &commands {
            tracing::info!("Deleting command {}", c.name);
            CONFIG
                .guild
                .delete_application_command(&ctx.http, c.id)
                .await?;
        }

        command
            .create_followup_message(&ctx.http, |resp| {
                resp.content(match commands.len() {
                    0 => "Removed no commands.".to_string(),
                    1 => format!("Removed command {}", commands[0].name),
                    _ => "Removed all commands. Please wait for the bot to add the commands back."
                        .to_string(),
                })
                .flags(interaction::MessageFlags::EPHEMERAL)
            })
            .await?;
        Ok(None)
    }
    
    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
