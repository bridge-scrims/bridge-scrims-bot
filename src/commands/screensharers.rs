use futures::future::join_all;
use serenity::{
    async_trait,
    client::Context,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction, id::UserId,
    },
};

use crate::consts::{self, CONFIG};

use super::Command;

pub struct Screensharers;

#[async_trait]
impl Command for Screensharers {
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

    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let screensharers = join_all(consts::DATABASE.get_screensharers().into_iter().map(
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
                    embed.title("Unfreeze leaderboard")
                        .description("List of every screenshare member that has unfrozen someone before and how many times they did it.")
                        .fields(screensharers)
                })
            })
        }).await?;

        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
