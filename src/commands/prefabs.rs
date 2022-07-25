use std::collections::HashMap;

use base64::decode;
use serde_json::value::Value;
use serenity::model::{
    application::command::{CommandOptionType, CommandPermissionType},
    Permissions,
};
use serenity::{
    async_trait,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType, MessageFlags,
    },
    prelude::Context,
    utils::Color,
};

use crate::commands::Command;
use crate::consts::CONFIG;

type Message = HashMap<String, Value>;
type Messages = HashMap<String, Vec<Message>>;
type Prefabs = HashMap<String, Messages>;

pub struct Prefab {
    prefabs: Prefabs,
}

#[async_trait]
impl Command for Prefab {
    fn name(&self) -> String {
        "prefab".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd = CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Sends a given prefab.")
                    .create_option(|o| {
                        o.name("name")
                            .description("Select the prefab that you would like to send.")
                            .required(true)
                            .kind(CommandOptionType::String);
                        for name in self.prefabs.keys() {
                            o.add_string_choice(name, name);
                        }
                        o
                    })
                    .default_member_permissions(Permissions::empty())
            })
            .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx, cmd.id, |p| {
                for role in &[CONFIG.support, CONFIG.trial_support, CONFIG.staff] {
                    p.create_permission(|perm| {
                        perm.kind(CommandPermissionType::Role)
                            .id(role.0)
                            .permission(true)
                    });
                }
                p
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
                r.interaction_response_data(|d| d.flags(MessageFlags::EPHEMERAL))
                    .kind(InteractionResponseType::DeferredChannelMessageWithSource)
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
                        .color(Color::new(0x1abc9c))
                })
            })
            .await?;

        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
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
