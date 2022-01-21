use std::sync::Arc;

use serenity::async_trait;
use serenity::client::Context;
use serenity::futures::stream::BoxStream;
use serenity::futures::StreamExt;
use serenity::http::Http;
use serenity::model::guild::Member;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandOptionType,
};
use serenity::model::interactions::InteractionResponseType;
use serenity::model::misc::Mentionable;
use serenity::model::prelude::InteractionApplicationCommandCallbackDataFlags;
use serenity::utils::Color;
use tokio::sync::Mutex;
use tokio::time::Duration;

use crate::commands::Command;
use crate::consts::*;

pub struct Council {
    inner: Arc<Inner>,
}

struct Inner {
    prime_council: Mutex<String>,
    private_council: Mutex<String>,
    premium_council: Mutex<String>,
}

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
        match command.data.options[0]
            .value
            .as_ref()
            .unwrap()
            .as_str()
            .unwrap()
        {
            "Prime" => {
                let prime_council = self.inner.prime_council.lock().await;
                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.create_embed(|e| {
                            e.title("Prime Council")
                                .description(prime_council.to_string())
                                .color(Color::new(0x74a8ee))
                        })
                    })
                    .await?;
            }
            "Private" => {
                let private_council = self.inner.private_council.lock().await;
                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.create_embed(|e| {
                            e.title("Private Council")
                                .description(private_council.to_string())
                                .color(Color::new(0xadade0))
                        })
                    })
                    .await?;
            }
            "Premium" => {
                let premium_council = self.inner.premium_council.lock().await;
                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.create_embed(|e| {
                            e.title("Premium Council")
                                .description(premium_council.to_string())
                                .color(Color::new(0xbb77fc))
                        })
                    })
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }

    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Council {
            inner: Arc::new(Inner {
                prime_council: Mutex::new("".into()),
                private_council: Mutex::new("".into()),
                premium_council: Mutex::new("".into()),
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
        tracing::info!("Updating councils");
        let mut prime_council_lock = self.prime_council.lock().await;
        let mut private_council_lock = self.private_council.lock().await;
        let mut premium_council_lock = self.premium_council.lock().await;
        let mut prime_head = "".to_string();
        let mut private_head = "".to_string();
        let mut premium_head = "".to_string();
        let mut prime_council = Vec::new();
        let mut private_council = Vec::new();
        let mut premium_council = Vec::new();
        let mut members: BoxStream<Member> = GUILD
            .members_iter(&http)
            .filter_map(|r| async move { r.ok() })
            .boxed();
        while let Some(member) = members.next().await {
            if member.roles.contains(&PRIME_HEAD) {
                prime_head = member.user.mention().to_string();
            } else if member.roles.contains(&PRIME_COUNCIL) {
                prime_council.push(member.user.mention().to_string());
            }
            if member.roles.contains(&PRIVATE_HEAD) {
                private_head = member.user.mention().to_string();
            } else if member.roles.contains(&PRIVATE_COUNCIL) {
                private_council.push(member.user.mention().to_string());
            }
            if member.roles.contains(&PREMIUM_HEAD) {
                premium_head = member.user.mention().to_string();
            } else if member.roles.contains(&PREMIUM_COUNCIL) {
                premium_council.push(member.user.mention().to_string());
            }
        }
        *prime_council_lock = format!(
            "{} - Council Head\n{}",
            prime_head,
            prime_council.join("\n")
        );
        *private_council_lock = format!(
            "{} - Council Head\n{}",
            private_head,
            private_council.join("\n")
        );
        *premium_council_lock = format!(
            "{} - Council Head\n{}",
            premium_head,
            premium_council.join("\n")
        );
    }
}
