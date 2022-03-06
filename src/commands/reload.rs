use bridge_scrims::interact_opts::InteractOpts;
use serenity::{
    async_trait,
    client::Context,
    model::interactions::{
        application_command::{
            ApplicationCommandInteraction, ApplicationCommandOptionType,
            ApplicationCommandPermissionType,
        },
        InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
    },
};

use crate::consts::CONFIG;

pub struct Reload;

#[async_trait]
impl super::Command for Reload {
    fn name(&self) -> String {
        String::from("reload")
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd = CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name())
                    .description("Reloads application commands.")
                    .default_permission(false)
                    .create_option(|opt| {
                        let opt = opt
                            .name("command")
                            .description("Which command to remove. Default: all")
                            .kind(ApplicationCommandOptionType::String)
                            .required(false);
                        for command in crate::handler::COMMANDS.iter() {
                            let name = command.name();
                            opt.add_string_choice(&name, &name);
                        }
                        opt
                    })
            })
            .await?;

        CONFIG
            .guild
            .create_application_command_permission(&ctx.http, cmd.id, |perm| {
                perm.create_permission(|perm| {
                    perm.kind(ApplicationCommandPermissionType::Role)
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
        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        let mut commands = CONFIG.guild.get_application_commands(&ctx.http).await?;

        if let Some(cmd) = command.get_str("command") {
            commands = commands.into_iter().filter(|x| x.name == cmd).collect();
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
                .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
            })
            .await?;
        Ok(())
    }
    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
