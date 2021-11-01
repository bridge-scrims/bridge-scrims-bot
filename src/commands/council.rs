use serenity::async_trait;
use serenity::client::Context;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandOptionType,
};

use crate::commands::Command;

pub struct Council;

#[async_trait]
impl Command for Council {
    fn name(&self) -> String {
        "council".to_string()
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::GUILD
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Lists the council members for a given council")
                    .create_option(|o| {
                        o.name("council")
                            .description("Available councils")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String)
                            .add_string_choice("Prime", "Prime")
                            .add_string_choice("Private", "Private")
                            .add_string_choice("Premium", "Premium")
                    })
            })
            .await?;
        Ok(())
    }

    async fn run(
        &self,
        _ctx: &Context,
        _command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        // I will work on this command -Shmill
        unimplemented!()
    }
}
