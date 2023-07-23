use std::error::Error;
use std::fmt::Display;

use serenity::{
    async_trait, builder::CreateEmbed, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::prelude::*, utils::Color,
};

use crate::{consts::CONFIG, handler::update_reactions_map};
use bridge_scrims::interaction::*;

pub struct Reaction;

pub struct DelReaction;

pub struct ListReactions;

#[derive(Debug)]
pub struct ReactionError {
    kind: ErrorKind,
}

impl Display for ReactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            match self.kind {
                ErrorKind::InvalidEmoji => "invalid emoji",
                ErrorKind::AlreadyExists => "reaction already exists",
                ErrorKind::Database => "database error",
                ErrorKind::InvalidTrigger => "invalid trigger",
            }
        )
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
impl InteractionHandler for DelReaction {
    fn name(&self) -> String {
        "delete_reaction".to_string()
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("A Staff Command to delete other users' custom reactions")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|user| {
                        user.kind(command::CommandOptionType::User)
                            .name("user")
                            .description("Whose reaction to delete")
                            .required(true)
                    })
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
                r.interaction_response_data(|d| d)
                    .kind(interaction::InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        let cmd = &command.data.options[0];
        let user = UserId(cmd.value.as_ref().unwrap().as_str().unwrap().parse()?)
            .to_user(&ctx.http)
            .await?;
        let user_id = user.id;

        if crate::consts::DATABASE
            .remove_custom_reaction(user_id.0)
            .await
            .is_err()
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
            }));
        }
        update_reactions_map().await;

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

        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(DelReaction {})
    }
}

#[async_trait]
impl InteractionHandler for Reaction {
    fn name(&self) -> String {
        "reaction".to_string()
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Allows server boosters to add their own custom reactions to the bot.")
                    .default_member_permissions(Permissions::empty())
                    .create_option(|add| {
                        add.kind(command::CommandOptionType::SubCommand)
                            .name("add")
                            .description("Add your own custom reaction!")
                            .create_sub_option(|emoji| {
                                // which emoji is it
                                emoji.kind(command::CommandOptionType::String)
                                    .name("emoji")
                                    .description("The emoji which the bot will react with (only default emojis allowed)")
                                    .required(true)
                            })
                            .create_sub_option(|trigger| {
                                // what will trigger the emoji to be reacted
                                trigger.kind(command::CommandOptionType::String)
                                    .name("trigger")
                                    .description("What will trigger the emoji to be reacted")
                                    .required(true)
                            })
                    })
                    .create_option(|rem| {
                        rem.kind(command::CommandOptionType::SubCommand)
                            .name("remove")
                            .description("Remove your custom reaction.")
                    })
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
                r.interaction_response_data(|d| d)
                    .kind(interaction::InteractionResponseType::DeferredChannelMessageWithSource)
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
                    }));
                }

                let reactions_with_trigger = crate::consts::DATABASE
                    .fetch_custom_reactions_with_trigger(&trigger)
                    .await?;

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
                    }));
                }
                let user_reactions = crate::consts::DATABASE
                    .fetch_custom_reactions_for(command.user.id.0)
                    .await?;

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
                    }));
                }
                // put it in the db, if there is an error let the user know that it didn't work
                if crate::consts::DATABASE
                    .add_custom_reaction(command.user.id.0, &emoji, &trigger)
                    .await
                    .is_err()
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
                    }));
                }
                update_reactions_map().await;

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
                if crate::consts::DATABASE
                    .remove_custom_reaction(command.user.id.0)
                    .await
                    .is_err()
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
                    }));
                }
                update_reactions_map().await;

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

        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(Reaction {})
    }
}

#[async_trait]
impl InteractionHandler for ListReactions {
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
        let reactions = crate::consts::DATABASE.fetch_custom_reactions().await?;

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
            return Ok(None);
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
                                        reaction.trigger, reaction.emoji, reaction.user_id
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
                                        reaction.trigger, reaction.emoji, reaction.user_id
                                    ),
                                    false,
                                );
                            }
                            e
                        })
                        .flags(interaction::MessageFlags::EPHEMERAL)
                    })
                    .await?;
            }
        }

        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(ListReactions {})
    }
}
