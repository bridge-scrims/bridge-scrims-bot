use crate::commands::Command;

use serenity::async_trait;
use serenity::client::Context;
use crate::interact_opts::InteractOpts;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandOptionType, ApplicationCommandPermissionType
};
use serenity::model::interactions::InteractionResponseType;
use serenity::utils::Color;
use serenity::model::id::UserId;
use serenity::model::channel::ReactionType;

use crate::consts::CONFIG;

pub struct Reaction;
pub struct DelReaction;

#[async_trait]
impl Command for DelReaction {
    fn name(&self) -> String {
        "deletereaction".to_string()
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()> {

        let cmd2 = CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name("deletereaction")
                .description("A Staff Command to delete other users' custom reactions")
                .default_permission(false)
                .create_option(|user| {
                    user.kind(ApplicationCommandOptionType::User)
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
        let cmd = &command.data.options[0];
        let user = UserId(cmd.value.as_ref().unwrap().as_str().unwrap().parse()?)
            .to_user(&ctx.http)
            .await?;
        let user_id = user.id;


        let mut code = 0;
        if let Err(_err) = crate::consts::DATABASE.remove_custom_reaction(
            user_id.0,
        ) {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.create_embed(|e| {
                        e.title("Reaction Could Not Be Removed!")
                            .description(format!("There was an error with the database! Try again, and if this happens again contact a developer."))
                            .color(Color::new(0x8b0000))
                    })
                })
                .await?;
                code = -1;
        }
        if code == 0 {
            command
                .edit_original_interaction_response(&ctx, |r| {
                    r.create_embed(|e| {
                        e.title("Reaction Removed")
                            .description(format!("<@{}>'s reaction has been removed.", user_id.0))
                            .color(Color::new(0x1abc9c)) // light green
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
                    .default_permission(false)
                    .create_option(|add| {
                        add.kind(ApplicationCommandOptionType::SubCommand)
                            .name("add")
                            .description("Add your own custom reaction!")
                            .create_sub_option(|emoji| {
                                // which emoji is it
                                emoji.kind(ApplicationCommandOptionType::String)
                                    .name("emoji")
                                    .description("The emoji which the bot will react with (only default emojis allowed)")
                                    .required(true)
                            })
                            .create_sub_option(|trigger| {
                                // what will trigger the emoji to be reacted
                                trigger.kind(ApplicationCommandOptionType::String)
                                    .name("trigger")
                                    .description("What will trigger the emoji to be reacted")
                                    .required(true)
                            })

                    })
                    .create_option(|rem| {
                        rem.kind(ApplicationCommandOptionType::SubCommand)
                            .name("remove")
                            .description("Remove your custom reaction.")
                    })



            })
            .await?;



            CONFIG
                .guild
                .create_application_command_permission(&ctx, cmd.id, |p| {

                    for role in &[CONFIG.server_booster] {
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
            let cmd = &command.data.options[0];
            match cmd.name.as_str() {
                "add" => {
                    // get user input
                    let emoji = cmd.get_str("emoji").unwrap();
                    let trigger = cmd.get_str("trigger").unwrap();

                    let mut code = 0;
                    // -1 = not valid emoji, -2 = already has a reaction, -3 = database error

                    let msg1 = command
                        .edit_original_interaction_response(&ctx, |r| {
                            r.create_embed(|e| {
                                e.title("Testing Reaction")
                                    .description(format!("The bot is currently testing your reaction to see if it is valid."))
                                    .color(Color::new(0x1abc9c))
                            })
                        })
                        .await?;

                        if let Err(_err) = msg1.react(&ctx, ReactionType::Unicode(String::from(&emoji))).await {
                            command
                                .edit_original_interaction_response(&ctx, |r| {
                                    r.create_embed(|e| {
                                        e.title("Reaction Could Not Be Added")
                                            .description(format!("{} is not a valid default emoji", &emoji))
                                            .color(Color::new(0x8b0000))
                                    })
                                }).await?;
                            code = -1;
                        }

                    if code == 0 {

                        let user_reactions = crate::consts::DATABASE.fetch_custom_reactions_for(command.user.id.0);

                        if user_reactions.len() > 0 {
                            command
                                .edit_original_interaction_response(&ctx, |r| {
                                    r.create_embed(|e| {
                                        e.title("Reaction Could Not Be Added")
                                            .description(format!("You already have a reaction! Remove it with `/reaction remove` and then try again."))
                                            .color(Color::new(0x8b0000))
                                    })
                                })
                                .await?;
                            code = -2;
                        }
                    }
                        // put it in the db, if there is an error let the user know that it didn't work
                        if code == 0 {
                            if let Err(_err) = crate::consts::DATABASE.add_custom_reaction(
                                command.user.id.0,
                                &emoji,
                                &trigger,
                            ) {
                                command
                                    .edit_original_interaction_response(&ctx, |r| {
                                        r.create_embed(|e| {
                                            e.title("Reaction Could Not Be Added")
                                                .description(format!("There was an error with the database! Try again, and if this happens again contact a developer."))
                                                .color(Color::new(0x8b0000))
                                        })
                                    })
                                    .await?;
                                code = -3;
                            }
                        }

                        if code == 0 {
                            command
                                .edit_original_interaction_response(&ctx, |r| {
                                    r.create_embed(|e| {
                                        e.title("Reaction Added")
                                            .description(format!("The {} reaction has been added.", &emoji))
                                            .color(Color::new(0x1abc9c)) // light green
                                    })
                                })
                                .await?;
                        }
                    },
                "remove" => {
                    let mut code = 0;
                    if let Err(_err) = crate::consts::DATABASE.remove_custom_reaction(
                        command.user.id.0,
                    ) {
                        command
                            .edit_original_interaction_response(&ctx, |r| {
                                r.create_embed(|e| {
                                    e.title("Reaction Could Not Be Removed!")
                                        .description(format!("There was an error with the database! Try again, and if this happens again contact a developer."))
                                        .color(Color::new(0x8b0000))
                                })
                            })
                            .await?;
                            code = -1;
                    }
                    if code == 0 {
                        command
                            .edit_original_interaction_response(&ctx, |r| {
                                r.create_embed(|e| {
                                    e.title("Reaction Removed")
                                        .description(format!("Your reaction has been removed."))
                                        .color(Color::new(0x1abc9c)) // light green
                                })
                            })
                            .await?;
                    }
                },
                "delete" => {

                }
                _ => {

                }
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
