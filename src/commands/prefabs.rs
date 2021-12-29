use crate::commands::Command;
use base64::decode;
use serde_json::value::Value;
use serenity::{
    async_trait,
    model::{
        interactions::{
            application_command::{
                ApplicationCommandInteraction, ApplicationCommandOptionType,
                ApplicationCommandPermissionType,
            },
            InteractionResponseType,
        },
        prelude::InteractionApplicationCommandCallbackDataFlags,
    },
    prelude::Context,
    utils::Color,
};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

type Message = HashMap<String, Value>;
type Messages = HashMap<String, Vec<Message>>;
type Prefabs = HashMap<String, Messages>;

pub struct Prefab {
    inner: Arc<Inner>,
}

struct Inner {
    prefabs: Prefabs,
}

#[async_trait]
impl Command for Prefab {
    fn name(&self) -> String {
        "prefab".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd = crate::GUILD
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Sends a given prefab.")
                    .create_option(|o| {
                        o.name("name")
                            .description("Select the prefab that you would like to send.")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String);
                        for (name, _) in &self.inner.prefabs {
                            o.add_string_choice(name, name);
                        }
                        o
                    })
                    .default_permission(false)
            })
            .await?;
        crate::GUILD
            .create_application_command_permission(&ctx, cmd.id, |p| {
                for role in vec![
                    *crate::consts::SUPPORT_ROLE,
                    *crate::consts::TRIAL_SUPPORT_ROLE,
                    *crate::consts::STAFF_ROLE,
                ] {
                    p.create_permission(|perm| {
                        perm.kind(ApplicationCommandPermissionType::Role)
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
                r.interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                })
                .kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;
        let s = command.data.options[0].value.clone().unwrap().to_string();
        let mut i = s.chars();
        i.next();
        i.next_back();
        let i = i.as_str();
        let m: Messages = self.inner.prefabs[i].clone();
        let member = ctx
            .http
            .get_member(crate::GUILD.0, command.user.id.0)
            .await
            .unwrap();
        let mut x = false;
        for role in vec![
            *crate::consts::SUPPORT_ROLE,
            *crate::consts::TRIAL_SUPPORT_ROLE,
            *crate::consts::STAFF_ROLE,
        ] {
            if member.roles.contains(&role) {
                x = true;
            }
        }
        if !x {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.create_embed(|e| {
                        e.title("Missing Permissions")
                            .description(
                                "You currently do **NOT** have permissions to do this command.",
                            )
                            .color(Color::DARK_RED)
                    })
                })
                .await?;

            return Ok(());
        }
        for message in &m["messages"] {
            let _ = &ctx
                .http
                .send_message(command.channel_id.0, &message["data"])
                .await;
        }
        command
            .edit_original_interaction_response(&ctx, |r| {
                r.create_embed(|e| {
                    e.title("Prefab Sent")
                        .description(format!("The prefab `{}` has been sent.", i))
                        .color(Color::MAGENTA)
                })
            })
            .await?;

        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        let data_str = fs::read_to_string("prefabs.json").unwrap();
        let data = serde_json::from_str::<HashMap<String, String>>(&data_str).unwrap();

        let mut prefabs: Prefabs = HashMap::new();
        for (prefab_name, prefab_value) in data {
            let s = decode(prefab_value).unwrap();
            let s = String::from_utf8(s).unwrap();
            let d = serde_json::from_str::<Messages>(&s).unwrap();
            prefabs.insert(prefab_name, d);
        }
        Box::new(Prefab {
            inner: Arc::new(Inner { prefabs }),
        })
    }
}
