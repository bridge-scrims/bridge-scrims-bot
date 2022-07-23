use std::error::Error;
use std::fmt::Display;

use serenity::async_trait;
use serenity::builder::CreateEmbed;
use serenity::client::Context;
use serenity::model::application::command::{CommandOptionType, CommandPermissionType};
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::application::interaction::MessageFlags;
use serenity::model::channel::ReactionType;
use serenity::model::id::UserId;
use serenity::model::Permissions;
use serenity::utils::Color;

use bridge_scrims::interact_opts::InteractOpts;

use crate::commands::Command;
use crate::consts::CONFIG;

pub struct Reaction;

pub struct DelReaction;

pub struct ListReactions;

#[derive(Debug)]
pub struct ReactionError {
    kind: ErrorKind,
    db_error: Option<sqlite::Error>,
}

impl Display for ReactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.kind {
                ErrorKind::InvalidEmoji => "invalid emoji",
                ErrorKind::AlreadyExists => "reaction already exists",
                ErrorKind::Database => "database error",
                ErrorKind::InvalidTrigger => "invalid trigger",
            }
        )?;
        if let Some(ref e) = self.db_error {
            writeln!(f, ": {}", e)
        } else {
            writeln!(f)
        }
    }
}

impl Error for ReactionError {}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorKind {
    /// Emoji is not valid
    InvalidEmoji = -1,
    /// User already has a reaction
    AlreadyExists = -2,
    /// Database error
    Database = -3,
    /// Trigger is not valid
    InvalidTrigger = -4,
}

#[async_trait]
impl Command for DelReaction {
    fn name(&self) -> String {
        "delete_reaction".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd2 = CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("A Staff Command to delete other users' custom reactions")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|user| {
                        user.kind(CommandOptionType::User)
                            .name("user")
                            .description("Whose reaction to delete")
                            .required(true)
                    })
            })
            .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx, cmd2.id, |p| {
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
                r.interaction_response_data(|d| d)
                    .kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;
        let cmd = &command.data.options[0];
        let user = UserId(cmd.value.as_ref().unwrap().as_str().unwrap().parse()?)
            .to_user(&ctx.http)
            .await?;
        let user_id = user.id;

        if let Err(db_error) = crate::consts::DATABASE.remove_custom_reaction(user_id.0) {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.embed(|e| {
                        e.title("Reaction Could Not Be Removed!")
                            .description("There was an error with the database! Try again, and if this happens again contact a developer.")
                            .color(Color::new(0x8b0000))
                    })
                })
                .await?;
            return Err(Box::new(ReactionError {
                kind: ErrorKind::InvalidEmoji,
                db_error: Some(db_error),
            }));
        }
        command
            .edit_original_interaction_response(&ctx, |r| {
                r.embed(|e| {
                    e.title("Reaction Removed")
                        .description(format!("<@{}>'s reaction has been removed.", user_id.0))
                        .color(Color::new(0x1abc9c)) // light green
                })
            })
            .await?;
        let mut embed = CreateEmbed::default();
        embed.title(format!("{}'s reaction has been removed", user.tag()));
        embed.description(format!(
            "Removed using the /delete_reaction command by {}",
            command.user.tag()
        ));
        if let Err(err) = CONFIG
            .reaction_logs
            .send_message(&ctx, |msg| msg.set_embed(embed.clone()))
            .await
        {
            tracing::error!("Error when sending message: {}", err);
        }

        Ok(())
    }

    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(DelReaction {})
    }
}

#[async_trait]
impl Command for Reaction {
    fn name(&self) -> String {
        "reaction".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let cmd = CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Allows server boosters to add their own custom reactions to the bot.")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|add| {
                        add.kind(CommandOptionType::SubCommand)
                            .name("add")
                            .description("Add your own custom reaction!")
                            .create_sub_option(|emoji| {
                                // which emoji is it
                                emoji.kind(CommandOptionType::String)
                                    .name("emoji")
                                    .description("The emoji which the bot will react with (only default emojis allowed)")
                                    .required(true)
                            })
                            .create_sub_option(|trigger| {
                                // what will trigger the emoji to be reacted
                                trigger.kind(CommandOptionType::String)
                                    .name("trigger")
                                    .description("What will trigger the emoji to be reacted")
                                    .required(true)
                            })
                    })
                    .create_option(|rem| {
                        rem.kind(CommandOptionType::SubCommand)
                            .name("remove")
                            .description("Remove your custom reaction.")
                    })
            })
            .await?;
        let mut brole = None;
        for (id, role) in CONFIG.guild.roles(&ctx.http).await? {
            if role.tags.premium_subscriber {
                brole = Some(id);
                break;
            }
        }
        CONFIG
            .guild
            .create_application_command_permission(&ctx, cmd.id, |p| {
                if let Some(id) = brole {
                    p.create_permission(|perm| {
                        perm.kind(CommandPermissionType::Role)
                            .id(id.0)
                            .permission(true)
                    });
                }
                p.create_permission(|perm| {
                    perm.kind(CommandPermissionType::Role)
                        .id(CONFIG.staff.0)
                        .permission(true)
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
                r.interaction_response_data(|d| d)
                    .kind(InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;
        let cmd = &command.data.options[0];
        match cmd.name.as_str() {
            "add" => {
                // get user input
                let emoji = cmd.get_str("emoji").unwrap();
                let trigger = cmd.get_str("trigger").unwrap();

                if trigger.to_ascii_lowercase().contains("ratio")
                    || trigger.to_ascii_lowercase().contains("shmill")
                    || trigger.starts_with('/')
                    || trigger.starts_with('<')
                {
                    command
                        .edit_original_interaction_response(&ctx, |r| {
                            r.embed(|e| {
                                e.title("Reaction Could Not Be Added")
                                    .description(format!(
                                        "You are not allowed to use `{}` as a trigger",
                                        &trigger
                                    ))
                                    .color(Color::new(0x8b0000))
                            })
                        })
                        .await?;
                    return Err(Box::new(ReactionError {
                        kind: ErrorKind::InvalidTrigger,
                        db_error: None,
                    }));
                }

                let reactions_with_trigger =
                    crate::consts::DATABASE.fetch_custom_reactions_with_trigger(&trigger);

                if !reactions_with_trigger.is_empty() {
                    command
                        .edit_original_interaction_response(&ctx, |r| {
                            r.embed(|e| {
                                e.title("Reaction Could Not Be Added")
                                    .description("That trigger already exists!")
                                    .color(Color::new(0x8b0000))
                            })
                        })
                        .await?;
                    return Err(Box::new(ReactionError {
                        kind: ErrorKind::InvalidTrigger,
                        db_error: None,
                    }));
                }

                let msg1 = command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.embed(|e| {
                            e.title("Testing Reaction")
                                .description("The bot is currently testing your reaction to see if it is valid.")
                                .color(Color::new(0x1abc9c))
                        })
                    })
                    .await?;

                if msg1
                    .react(&ctx, ReactionType::try_from(emoji.as_str()).unwrap())
                    .await
                    .is_err()
                {
                    command
                        .edit_original_interaction_response(&ctx, |r| {
                            r.embed(|e| {
                                e.title("Reaction Could Not Be Added")
                                    .description(format!("{} is not a valid default emoji", &emoji))
                                    .color(Color::new(0x8b0000))
                            })
                        })
                        .await?;
                    return Err(Box::new(ReactionError {
                        kind: ErrorKind::InvalidEmoji,
                        db_error: None,
                    }));
                }
                let user_reactions =
                    crate::consts::DATABASE.fetch_custom_reactions_for(command.user.id.0);

                if !user_reactions.is_empty() {
                    command
                        .edit_original_interaction_response(&ctx, |r| {
                            r.embed(|e| {
                                e.title("Reaction Could Not Be Added")
                                    .description("You already have a reaction! Remove it with `/reaction remove` and then try again.")
                                    .color(Color::new(0x8b0000))
                            })
                        })
                        .await?;
                    return Err(Box::new(ReactionError {
                        kind: ErrorKind::AlreadyExists,
                        db_error: None,
                    }));
                }
                // put it in the db, if there is an error let the user know that it didn't work
                if let Err(db_error) =
                    crate::consts::DATABASE.add_custom_reaction(command.user.id.0, &emoji, &trigger)
                {
                    command
                        .edit_original_interaction_response(&ctx, |r| {
                            r.embed(|e| {
                                e.title("Reaction Could Not Be Added")
                                    .description("There was an error with the database! Try again, and if this happens again contact a developer.")
                                    .color(Color::new(0x8b0000))
                            })
                        })
                        .await?;
                    return Err(Box::new(ReactionError {
                        kind: ErrorKind::Database,
                        db_error: Some(db_error),
                    }));
                }

                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.embed(|e| {
                            e.title("Reaction Added")
                                .description(format!("The {} reaction has been added.", &emoji))
                                .color(Color::new(0x1abc9c)) // light green
                        })
                    })
                    .await?;

                let mut embed = CreateEmbed::default();
                embed.title(format!("New reaction added by {}.", command.user.tag()));
                embed.description(format!("`{}` reacts with `{}`", &trigger, &emoji));
                if let Err(err) = CONFIG
                    .reaction_logs
                    .send_message(&ctx, |msg| msg.set_embed(embed.clone()))
                    .await
                {
                    tracing::error!("Error when sending message: {}", err);
                }
            }
            "remove" => {
                if let Err(db_error) =
                    crate::consts::DATABASE.remove_custom_reaction(command.user.id.0)
                {
                    command
                        .edit_original_interaction_response(&ctx, |r| {
                            r.embed(|e| {
                                e.title("Reaction Could Not Be Removed!")
                                    .description("There was an error with the database! Try again, and if this happens again contact a developer.")
                                    .color(Color::new(0x8b0000))
                            })
                        })
                        .await?;
                    return Err(Box::new(ReactionError {
                        kind: ErrorKind::Database,
                        db_error: Some(db_error),
                    }));
                }
                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.embed(|e| {
                            e.title("Reaction Removed")
                                .description("Your reaction has been removed.")
                                .color(Color::new(0x1abc9c)) // light green
                        })
                    })
                    .await?;
                let mut embed = CreateEmbed::default();
                embed.title(format!(
                    "{}'s reaction has been removed",
                    command.user.tag()
                ));
                embed.description("Used the /reaction remove command.");
                if let Err(err) = CONFIG
                    .reaction_logs
                    .send_message(&ctx, |msg| msg.set_embed(embed.clone()))
                    .await
                {
                    tracing::error!("Error when sending message: {}", err);
                }
            }
            _ => {}
        }

        Ok(())
    }
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(Reaction {})
    }
}

#[async_trait]
impl Command for ListReactions {
    fn name(&self) -> String {
        "list_reactions".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description(
                        "Sends a list of all reactions, their triggers, and associated users.",
                    )
                    .default_member_permissions(Permissions::empty())
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
        let reactions = crate::consts::DATABASE.fetch_custom_reactions();

        if reactions.is_empty() {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.embed(|e| {
                        e.title("No custom reactions found!")
                            .description("There are currently no custom reactions.")
                            .color(Color::BLURPLE)
                    })
                })
                .await?;
            return Ok(());
        }
        for (i, chunk) in reactions.chunks(10).enumerate() {
            if i == 0 {
                command
                    .edit_original_interaction_response(&ctx, |r| {
                        r.embed(|e| {
                            e.title(format!(
                                "Page {} of {}",
                                i + 1,
                                ((reactions.len() - 1) / 10) + 1
                            ))
                            .description("These are all custom reactions currently:")
                            .color(Color::BLURPLE);
                            for reaction in chunk {
                                e.field(
                                    format!("Reaction `{}`:", reaction.trigger),
                                    format!(
                                        "`{}` reacts with {}, created by <@!{}>",
                                        reaction.trigger, reaction.emoji, reaction.user
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
                            e.title(format!("Page {} of {}", i + 1, (reactions.len() / 10) + 1))
                                .color(Color::BLURPLE);
                            for reaction in chunk {
                                e.field(
                                    format!("Reaction `{}`:", reaction.trigger),
                                    format!(
                                        "`{}` reacts with {}, created by <@!{}>",
                                        reaction.trigger, reaction.emoji, reaction.user
                                    ),
                                    false,
                                );
                            }
                            e
                        })
                        .flags(MessageFlags::EPHEMERAL)
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
        Box::new(ListReactions {})
    }
}
