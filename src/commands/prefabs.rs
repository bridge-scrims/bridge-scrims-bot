use std::collections::HashMap;

use base64::decode;
use serde_json::value::Value;

use serenity::{
    async_trait, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::prelude::*,
};

use crate::consts::CONFIG;
use bridge_scrims::interaction::*;

type Message = HashMap<String, Value>;
type Messages = HashMap<String, Vec<Message>>;
type Prefabs = HashMap<String, Messages>;

pub struct Prefab {
    prefabs: Prefabs,
}

#[async_trait]
impl InteractionHandler for Prefab {
    fn name(&self) -> String {
        "prefab".to_string()
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Sends a given prefab.")
                    .create_option(|o| {
                        o.name("name")
                            .description("Select the prefab that you would like to send.")
                            .required(true)
                            .kind(command::CommandOptionType::String);
                        for name in self.prefabs.keys() {
                            o.add_string_choice(name, name);
                        }
                        o
                    })
                    .default_member_permissions(Permissions::empty())
            })
            .await?;
        Ok(())
    }

    async fn handle_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        command
            .create_interaction_response(&ctx, |r| {
                r.interaction_response_data(|d| d.flags(interaction::MessageFlags::EPHEMERAL))
                    .kind(interaction::InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;
        let s = command.data.options[0]
            .value
            .as_ref()
            .unwrap()
            .as_str()
            .unwrap();
        let m: Messages = self.prefabs[s].clone();
        for message in &m["messages"] {
            let _ = &ctx
                .http
                .send_message(command.channel_id.0, &message["data"])
                .await;
        }
        command
            .edit_original_interaction_response(&ctx, |r| {
                r.embed(|e| {
                    e.title("Prefab Sent")
                        .description(format!("The prefab `{}` has been sent.", s))
                        .color(0x1abc9c)
                })
            })
            .await?;

        Ok(None)
    }

    fn new() -> Box<Self> {
        let data = &CONFIG.prefabs;

        let mut prefabs: Prefabs = HashMap::new();
        for (prefab_name, prefab_value) in data {
            let s = decode(prefab_value).unwrap();
            let s = String::from_utf8(s).unwrap();
            let d = serde_json::from_str::<Messages>(&s).unwrap();
            prefabs.insert(prefab_name.to_string(), d);
        }
        Box::new(Prefab { prefabs })
    }
}
