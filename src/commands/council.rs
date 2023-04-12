use std::collections::HashMap;
use std::sync::Arc;

use tokio::{sync::Mutex, time::Duration};
use futures::StreamExt;

use serenity::{
    async_trait,
    client::Context,
    http::Http,

    model::prelude::*,
    model::application::interaction::application_command::ApplicationCommandInteraction
};

use bridge_scrims::interaction::*;
use crate::consts::CONFIG;

pub struct Council {
    councils: Arc<Inner>,
}

#[async_trait]
impl InteractionHandler for Council {

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
                            .kind(command::CommandOptionType::String);
                        for name in CONFIG.councils.keys() {
                            o.add_string_choice(name, name);
                        }
                        o
                    })
            })
            .await?;
        Ok(())
    }

    async fn handle_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult
    {
        command
            .create_interaction_response(&ctx, |r| {
                r.interaction_response_data(|d| d.flags(interaction::MessageFlags::EPHEMERAL))
                    .kind(interaction::InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;
        let name = command.get_str("council").unwrap();

        if let Some(value) = self.councils.get_council(&name).await {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.embed(|e| {
                        e.title(format!("{} Council", name))
                            .description(value)
                            .color(0xbb77fc)
                    })
                })
                .await?;
        } else {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.embed(|e| {
                        e.title("Invalid Council")
                            .description("The councils are not yet loaded!")
                            .color(0xbb77fc)
                    })
                })
                .await?;
        }
        Ok(None)
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
        (*councils).get(name).cloned()
    }
    
}
