use futures::future::join_all;
use serenity::{
    async_trait,
    client::Context,

    model::prelude::*,
    model::application::interaction::application_command::ApplicationCommandInteraction
};

use bridge_scrims::interaction::*;
use crate::consts::{CONFIG, DATABASE};

pub struct Screensharers;

#[async_trait]
impl InteractionHandler for Screensharers {

    fn name(&self) -> String {
        String::from("screensharers")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name()).description(
                    "Lists the screenshare team and how much they've unfrozen someone.",
                )
            })
            .await?;
        Ok(())
    }

    async fn handle_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult
    {
        let screensharers = join_all(DATABASE.get_screensharers().into_iter().map(
            |x| async move {
                let user = UserId(x.id).to_user(&ctx.http).await;
                if let Ok(user) = user {
                    Some((user.tag(), format!("{} freezes", x.freezes), false))
                } else {
                    None
                }
            },
        ))
        .await
        .into_iter()
        .flatten();

        command.create_interaction_response(&ctx.http, |resp| {
            resp.interaction_response_data(|data| {
                data.embed(|embed| {
                    embed.title("Unfreeze Leaderboard")
                        .description("List of every screenshare member that has unfrozen someone before and how many times they did it.")
                        .fields(screensharers)
                })
            })
        }).await?;

        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
