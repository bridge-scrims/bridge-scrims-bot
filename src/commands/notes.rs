use time::OffsetDateTime;

use serenity::{
    async_trait,
    client::Context,
    utils::Color,

    model::prelude::*,
    model::application::command::CommandOptionType,
    model::application::interaction::application_command::ApplicationCommandInteraction
};

use bridge_scrims::interaction::*;
use crate::consts::CONFIG;

pub struct Notes;

#[async_trait]
impl InteractionHandler for Notes {

    fn name(&self) -> String {
        "notes".to_string()
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("A way for staff to set notes for users.")
                    // Sub command Options:
                    .create_option(|list| {
                        // list
                        list.kind(CommandOptionType::SubCommand)
                            .name("list")
                            .description("The notes for a given user.")
                            .create_sub_option(|opt| {
                                opt.kind(CommandOptionType::User)
                                    .name("user")
                                    .description("The user who's notes to retrieve.")
                                    .required(true)
                            })
                    })
                    .create_option(|add| {
                        add.kind(CommandOptionType::SubCommand)
                            .name("add")
                            .description("Add a note for a given user.")
                            .create_sub_option(|opt| {
                                opt.kind(CommandOptionType::User)
                                    .name("user")
                                    .description("The user to add a note to.")
                                    .required(true)
                            })
                            .create_sub_option(|opt| {
                                opt.kind(CommandOptionType::String)
                                    .name("note")
                                    .description("The note to add.")
                                    .required(true)
                            })
                    })
                    .create_option(|del| {
                        del.kind(CommandOptionType::SubCommand)
                            .name("remove")
                            .description("Delete a note from a user.")
                            .create_sub_option(|opt| {
                                opt.kind(CommandOptionType::User)
                                    .name("user")
                                    .description("The user to remove a note from.")
                                    .required(true)
                            })
                            .create_sub_option(|opt| {
                                opt.kind(CommandOptionType::Integer)
                                    .name("noteid")
                                    .description("The note id to delete.")
                                    .required(true)
                            })
                    })
                    .default_member_permissions(Permissions::empty())
            })
            .await?;
        Ok(())
    }

    async fn handle_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> InteractionResult
    {
        command
            .create_interaction_response(&ctx, |r| {
                r.interaction_response_data(|d| d)
                    .kind(interaction::InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;
        let cmd = &command.data.options[0];
        match cmd.name.as_str() {
            "list" => {
                let user = UserId(cmd.get_str("user").unwrap().parse()?)
                    .to_user(&ctx.http)
                    .await?;
                let user_id = user.id;
                let notes = crate::consts::DATABASE.fetch_notes_for(user_id.0);

                if notes.is_empty() {
                    command
                        .edit_original_interaction_response(&ctx, |r| {
                            r.embed(|e| {
                                e.title("No notes found!")
                                    .description(format!(
                                        "<@{}> currently has no notes.",
                                        user_id.0
                                    ))
                                    .color(Color::BLURPLE)
                            })
                        })
                        .await?;
                    return Ok(None);
                }
                for (i, chunk) in notes.chunks(10).enumerate() {
                    if i == 0 {
                        command
                            .edit_original_interaction_response(&ctx, |r| {
                                r.embed(|e| {
                                    e.title(format!(
                                        "Page {} of {}",
                                        i + 1,
                                        (notes.len() / 10) + 1
                                    ))
                                    .description(format!(
                                        "<@{}> currently the following notes:",
                                        user_id.0
                                    ))
                                    .color(Color::BLURPLE);
                                    for note in chunk {
                                        e.field(
                                            format!("Note {}:", note.id),
                                            format!(
                                                "<t:{}>: `{}` by <@!{}>",
                                                note.created_at.unix_timestamp(),
                                                note.note,
                                                note.creator
                                            ),
                                            false,
                                        );
                                    }
                                    e
                                })
                            })
                            .await?;
                    } else {
                        command
                            .create_followup_message(&ctx, |r| {
                                r.embed(|e| {
                                    e.title(format!(
                                        "Page {} of {}",
                                        i + 1,
                                        (notes.len() / 10) + 1
                                    ))
                                    .color(Color::BLURPLE);
                                    for note in chunk {
                                        e.field(
                                            format!("Note {}:", note.id),
                                            format!(
                                                "<t:{}>: `{}` by <@!{}>",
                                                note.created_at.unix_timestamp(),
                                                note.note,
                                                note.creator
                                            ),
                                            false,
                                        );
                                    }
                                    e
                                })
                            })
                            .await?;
                    }
                }
            }
            "add" => {
                let user = UserId(cmd.get_str("user").unwrap().parse()?)
                    .to_user(&ctx.http)
                    .await?;
                let user_id = user.id;

                let now = OffsetDateTime::now_utc();
                let note = cmd.get_str("note").unwrap();

                let noteid =
                    crate::consts::DATABASE.add_note(user_id.0, now, &note, command.user.id.0)?;
                if noteid == -1 {
                    command
                        .edit_original_interaction_response(&ctx, |r| {
                            r.embed(|e| {
                                e.title("Database Error!")
                                    .description(
                                        "There was an error when communicating with the database.",
                                    )
                                    .color(Color::DARK_RED)
                            })
                        })
                        .await?;
                }
                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.embed(|e| {
                            e.title("Note Added")
                                .description(format!(
                                    "The note `{}` has been added to <@{}> with id {}.",
                                    note, user_id.0, noteid
                                ))
                                .color(Color::BLURPLE)
                        })
                    })
                    .await?;
            }
            "remove" => {
                let user = UserId(cmd.get_str("user").unwrap().parse()?)
                    .to_user(&ctx.http)
                    .await?;
                let user_id = user.id;

                let noteid = cmd.get_u64("noteid").unwrap();

                crate::consts::DATABASE.remove_note(user_id.0, noteid)?;

                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.embed(|e| {
                            e.title("Note Removed")
                                .description(format!(
                                    "The note has been deleted from <@{}> with id {}.",
                                    user_id.0, noteid
                                ))
                                .color(Color::BLURPLE)
                        })
                    })
                    .await?;
            }
            _ => {
                // Do nothing since this wont happen.
            }
        }

        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(Notes {})
    }
}
