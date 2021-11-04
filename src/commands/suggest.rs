use std::sync::Arc;

use serenity::async_trait;
use serenity::client::Context;
use serenity::http::Http;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandOptionType,
};
use serenity::model::id::ChannelId;
use serenity::model::interactions::InteractionResponseType;
use serenity::model::prelude::InteractionApplicationCommandCallbackDataFlags;
use serenity::utils::Color;
use tokio::time::Duration;
use serenity::model::interactions::application_command::ApplicationCommandInteractionDataOptionValue;
use crate::commands::Command;

const SUGGESTIONS_CHANNEL: u64 = 905110434410016778;

pub struct Suggestion {
    inner: Arc<Inner>,
}

struct Inner {
}

#[async_trait]
impl Command for Suggestion {
    fn name(&self) -> String {
        "suggestion".to_string()
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::GUILD
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Suggest bridge scrims a suggestion")
                    .create_option(|o| {
                        o.name("suggestion:")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String)
                    })
            })
            .await?;
        tokio::spawn(update_loop(self.inner.clone(), ctx.http.clone()));
        Ok(())
    }

    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        command
            .create_interaction_response(&ctx, |r| {
                r.interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                })
                .kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        
        // Embed

        command
            .edit_original_interaction_response(&ctx, |r| {
                r.create_embed(|e| {
                    e.title("Suggestions")
                        .description("Your suggestion has been delivered successfully".to_string())
                        .color(Color::new(0x74a8ee))
                })
            })
            .await?;

        let s = match &command.data.options[0].resolved{
          Some(ApplicationCommandInteractionDataOptionValue::String(s)) => s.clone(),
          _ => panic!("expected a string value"),
        };
        ChannelId(SUGGESTIONS_CHANNEL).say(ctx, format!("received suggestion: {}", s)).await?;
        Ok(())
    }

    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Suggestion {
            inner: Arc::new(Inner {
            }),
        })
    }
}

async fn update_loop(inner: Arc<Inner>, http: Arc<Http>) {
    loop {
        inner.update(http.clone()).await;
        tokio::time::sleep(Duration::from_secs(21600)).await;
    }
}

impl Inner {
    async fn update(&self, http: Arc<Http>) {
        tracing::info!("Updating something");

    }
}
