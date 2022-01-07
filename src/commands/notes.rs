use crate::commands::Command;
use serenity::{
    async_trait,
    model::interactions::{
        application_command::{
            ApplicationCommandInteraction, ApplicationCommandOptionType,
            ApplicationCommandPermissionType,
        },
        InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
    },
    prelude::Context,
    utils::Color,
};

pub struct Notes;

#[async_trait]
impl Command for Notes {
    fn name(&self) -> String {
        "notes".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd = crate::GUILD
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("A way for staff to set notes for users.")
                    // Sub command Options:
                    .create_option(|list| {
                        // list
                        list.kind(ApplicationCommandOptionType::SubCommand)
                            .name("list")
                            .description("The notes for a given user.")
                            .create_sub_option(|opt| {
                                opt.kind(ApplicationCommandOptionType::User)
                                    .name("user")
                                    .description("The user who's notes to retrieve.")
                                    .required(true)
                            })
                    })
                    .create_option(|add| {
                        add.kind(ApplicationCommandOptionType::SubCommand)
                            .name("add")
                            .description("Add a note for a given user.")
                            .create_sub_option(|opt| {
                                opt.kind(ApplicationCommandOptionType::User)
                                    .name("user")
                                    .description("The user to add a note to.")
                                    .required(true)
                            })
                            .create_sub_option(|opt| {
                                opt.kind(ApplicationCommandOptionType::String)
                                    .name("note")
                                    .description("The note to add.")
                                    .required(true)
                            })
                    })
                    .create_option(|del| {
                        del.kind(ApplicationCommandOptionType::SubCommand)
                            .name("delete")
                            .description("Delete a note from a user.")
                            .create_sub_option(|opt| {
                                opt.kind(ApplicationCommandOptionType::Integer)
                                    .name("noteid")
                                    .description("The note id to delete.")
                                    .required(true)
                            })
                    })
                    .default_permission(false)
            })
            .await?;
        crate::GUILD
            .create_application_command_permission(&ctx, cmd.id, |p| {
                for role in &[
                    *crate::consts::SUPPORT,
                    *crate::consts::TRIAL_SUPPORT,
                    *crate::consts::STAFF,
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
                r.interaction_response_data(|d| d)
                    .kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;
        let member = ctx
            .http
            .get_member(crate::GUILD.0, command.user.id.0)
            .await
            .unwrap();
        let mut x = false;
        for role in &[
            *crate::consts::SUPPORT,
            *crate::consts::TRIAL_SUPPORT,
            *crate::consts::STAFF,
        ] {
            if member.roles.contains(role) {
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
        let cmd = &command.data.options[0];
        match cmd.name.as_str() {
            "list" => {
                // implement the list command
                println!("list command says hello")
            }
            "add" => {
                println!("add command says hello")
                // implement the add command
            }
            "delete" => {
                println!("delete command says hello")
                // implement the delete command
            }
            _ => {
                // Tell them something went wrong
                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.create_embed(|e| {
                            e.title("Something broke")
                                .description(format!(
                                    "You selected a sub command that doesn't exsist: {}",
                                    cmd.name
                                ))
                                .color(Color::DARK_RED)
                        })
                    })
                    .await?;
            }
        }

        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Notes {})
    }
}
