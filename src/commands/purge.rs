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

use tokio::sync::Mutex;
use tokio::time::Duration;

use crate::commands::Command;
use crate::consts::*;

pub struct Purge {
    inner: Arc<Inner>,
}

struct Inner {
    prime_council: Mutex<String>,
    private_council: Mutex<String>,
    premium_council: Mutex<String>,
}

#[async_trait]
impl Command for Purge {
    fn name(&self) -> String {
        "purge".to_string()
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::GUILD
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Purges a specific amount of messages from the channel")
                    .create_option(|o| {
                        o.name("number")
                            .description("Amount of messages to purge")
                            .required(true)
                            .kind(ApplicationCommandOptionType::Number)
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
        command
            .edit_original_interaction_response(&ctx.http, |r|{
            r.content("Purge Successfull!".to_string())
            })
            .await?;




        let channel = command.channel_id;
        let mut messages = channel.messages_iter(&ctx.http).boxed();
        
        let mut astr = command.data.options[0].value.as_ref().unwrap().as_i64().unwrap();



        while let Some(message_result) = messages.next().await {


            if astr.to_string() ==  "0" {
              break;
            }

            match message_result {
                Ok(message) =>{ 
                
                    
                    message.delete(&ctx.http).await?;
                    astr -= 1;
                },

                Err(why) => println!("Failed to get messages: {:?}", why),
            }
            
          }
        
        Ok(())
    }

    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Purge {
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
                prime_head = member.user.mention().to_string()
            } else if member.roles.contains(&PRIME_COUNCIL) {
                prime_council.push(member.user.mention().to_string())
            }
            if member.roles.contains(&PRIVATE_HEAD) {
                private_head = member.user.mention().to_string()
            } else if member.roles.contains(&PRIVATE_COUNCIL) {
                private_council.push(member.user.mention().to_string())
            }
            if member.roles.contains(&PREMIUM_HEAD) {
                premium_head = member.user.mention().to_string()
            } else if member.roles.contains(&PREMIUM_COUNCIL) {
                premium_council.push(member.user.mention().to_string())
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
