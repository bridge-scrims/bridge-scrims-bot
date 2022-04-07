use std::collections::HashMap;
use std::sync::Arc;

use bridge_scrims::interact_opts::InteractOpts;
use serenity::async_trait;
use serenity::client::Context;
use serenity::futures::StreamExt;
use serenity::http::Http;
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
use crate::consts::CONFIG;

pub struct Council {
    councils: Arc<Inner>,
}

#[async_trait]
impl Command for Council {
    fn name(&self) -> String {
        "council".to_string()
    }
    async fn init(&self, ctx: &Context) {
        tokio::spawn(Inner::update_loop(self.councils.clone(), ctx.http.clone()));
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Lists the council members for a given council")
                    .create_option(|o| {
                        o.name("council")
                            .description("The council who's members to display")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String);
                        for name in CONFIG.councils.keys() {
                            o.add_string_choice(
                                name,
                                format!("Display the members of the {} council.", name),
                            );
                        }
                        o
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
            .create_interaction_response(&ctx, |r| {
                r.interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                })
                .kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;
        let name = command.get_str("council").unwrap();
        tracing::info!("doing stuff...");
        if let Some(value) = self.councils.get_council(&name).await {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.create_embed(|e| {
                        e.title(format!("{} Council", name))
                            .description(value)
                            .color(Color::new(0xbb77fc))
                    })
                })
                .await?;
        } else {
            tracing::error!("Council not found {}, {}", name, self.councils.0.lock().await);
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.create_embed(|e| {
                        e.title("Invalid Council")
                            .description("An error has been detected!")
                            .color(Color::new(0xbb77fc))
                    })
                })
                .await?;
        }
        Ok(())
    }

    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Council {
            councils: Arc::new(Inner::new()),
        })
    }
}

pub struct Inner(pub Mutex<HashMap<String, String>>);

impl Inner {
    pub fn new() -> Inner {
        Inner(Mutex::new(HashMap::new()))
    }
    pub async fn update_loop(me: Arc<Inner>, http: Arc<Http>) {
        loop {
            me.clone().update_councils(http.clone()).await;
            // update every 12 hours
            tokio::time::sleep(Duration::from_secs(12 * 60 * 60)).await;
        }
    }
    pub async fn update_councils(&self, http: Arc<Http>) {
        let mut new: HashMap<String, String> = HashMap::new();
        let mut members = CONFIG.guild.members_iter(&http).boxed();
        while let Some(member) = members.next().await {
            if let Err(err) = member {
                tracing::info!("Error while updating councils: {}", err);
                continue;
            }
            let member = member.unwrap();
            for (name, council) in
                CONFIG.councils.clone().into_iter().filter(|(_k, v)| {
                    member.roles.contains(&v.role) || member.roles.contains(&v.head)
                })
            {
                let mut x = new.get(&name).cloned().unwrap_or_default();
                if member.roles.contains(&council.head) {
                    x = format!(
                        "{} ({}) - Council Head\n{}",
                        member.user.mention(),
                        member.display_name(),
                        x
                    );
                } else {
                    x = format!(
                        "{}\n{} ({})",
                        x,
                        member.user.mention(),
                        member.display_name()
                    );
                }
                new.insert(name, x);
            }
        }
        let mut c = self.0.lock().await;
        *c = new;
    }
    pub async fn get_council(&self, name: &str) -> Option<String> {
        let councils = self.0.lock().await;
        councils.get(name).cloned()
    }
}
