use bridge_scrims::interact_opts::InteractOpts;
use serenity::{
    async_trait,
    builder::CreateEmbed,
    client::Context,
    http::Http,
    model::{
        guild::Ban,
        id::UserId,
        interactions::{
            application_command::{
                ApplicationCommandInteraction, ApplicationCommandOptionType,
                ApplicationCommandPermissionType,
            },
            InteractionApplicationCommandCallbackDataFlags,
        },
    },
};

use crate::consts::CONFIG;

use super::Command;

#[derive(Debug)]
pub enum UnbanType {
    Scrim,
    Server,
}

pub enum UnbanEntry {
    Scrim(crate::model::ScrimUnban),
    Server(Ban),
}

impl UnbanType {
    pub async fn exec(
        &self,
        http: &Http,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let user = command.get_str("user").unwrap();
        let reason = command
            .get_str("reason")
            .unwrap_or_else(|| String::from("No reason given."));
        let user_id = UserId(user.parse().unwrap_or_default());

        let entry = match self {
            UnbanType::Scrim => crate::consts::DATABASE
                .fetch_scrim_unbans()
                .into_iter()
                .find(|x| x.id == user_id.0)
                .map(UnbanEntry::Scrim),
            UnbanType::Server => {
                let bans = CONFIG.guild.bans(&http).await?;
                bans.into_iter()
                    .find(|x| x.user.id == user_id || user.starts_with(&x.user.tag()))
                    .map(UnbanEntry::Server)
            }
        };
        if entry.is_none() {
            command
                .create_interaction_response(&http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content(format!("Could not find {} in our bans", user))
                    })
                })
                .await?;
            tracing::error!("Could not find {} in {:?}", user, self);
            return Ok(());
        }

        let res = self
            .unban(http, Some(command.user.id), entry.unwrap(), reason)
            .await;

        if let Err(ref e) = res {
            command
                .create_interaction_response(&http, |resp| {
                    resp.interaction_response_data(|data| {
                        data.content(format!("Could not unban {}: {}", user_id, e))
                            .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                    })
                })
                .await?;
        } else if command.channel_id != CONFIG.support_bans {
            command
                .create_interaction_response(&http, |resp| {
                    resp.interaction_response_data(|data| data.add_embed(res.unwrap()))
                })
                .await?;
        }

        Ok(())
    }
    pub async fn unban(
        &self,
        http: &Http,
        staff_id: Option<UserId>, // If staff id is not provided, it is assumed that the ban expired
        unban_entry: UnbanEntry,
        reason: String,
    ) -> Result<CreateEmbed, Box<dyn std::error::Error + Sync + Send>> {
        let to_unban = match unban_entry {
            UnbanEntry::Scrim(entry) => CONFIG.guild.member(&http, entry.id).await.unwrap().user,
            UnbanEntry::Server(entry) => entry.user.clone(),
        };

        let mut embed = CreateEmbed::default();
        embed.title(format!("{} was unbanned", to_unban.tag()));
        embed.field("User", format!("<@{}>", to_unban.id), false);
        if staff_id.is_some() {
            embed.field("Staff", format!("<@{}>", staff_id.unwrap()), false);
        }

        embed.field("Reason", format!("`{}`", reason), false);

        let mut result = Ok(());
        let mut db_result = Ok(());
        match self {
            Self::Server => {
                result = result.and(CONFIG.guild.unban(&http, to_unban.id).await);
                if result.is_ok() {
                    // Permanent server bans are not in the database, do not error if that is the
                    // case
                    let _ = crate::consts::DATABASE.remove_entry("ScheduledUnbans", to_unban.id.0);
                }
            }
            Self::Scrim => {
                let mut member = match CONFIG.guild.member(&http, to_unban.id).await {
                    Ok(memb) => memb,
                    Err(e) => return Err(Box::new(e)),
                };
                let unban = crate::consts::DATABASE
                    .fetch_scrim_unbans()
                    .into_iter()
                    .find(|x| x.id == to_unban.id.0)
                    .unwrap();
                let roles: Vec<_> = unban.roles.into();
                result = result.and(member.add_roles(&http, &roles).await.map(|_| ()));
                result = result.and(member.remove_role(&http, CONFIG.banned.0).await);

                // If roles cannot be added, don't remove the unban from the database either
                if result.is_ok() {
                    db_result =
                        crate::consts::DATABASE.remove_entry("ScheduledScrimUnbans", to_unban.id.0);
                }
            }
        }
        if result.is_ok() {
            CONFIG
                .support_bans
                .send_message(&http, |msg| msg.set_embed(embed.clone()))
                .await?;
        }
        if let Err(e) = db_result.as_ref() {
            embed.description(format!(
                "WARNING: the database responded with an error: {}",
                e
            ));
        }
        result?;
        Ok(embed)
    }
}

pub struct Unban;

#[async_trait]
impl Command for Unban {
    fn name(&self) -> String {
        String::from("unban")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = CONFIG.guild
            .create_application_command(&ctx, |c| {
                c
                    .name(self.name())
                    .description("Removes a ban from the given user in this server. This does not affect the \"Banned\" Role")
                    .default_permission(false)
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to unban")
                            .required(true)
                            .kind(ApplicationCommandOptionType::String)
                    })
                    .create_option(|o| {
                        o.name("reason")
                            .description("The reason to remove this user's ban")
                            .kind(ApplicationCommandOptionType::String)
                            .required(false)
                    })

            })
        .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
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
        UnbanType::Server.exec(&ctx.http, command).await?;
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

pub struct ScrimUnban;

#[async_trait]
impl Command for ScrimUnban {
    fn name(&self) -> String {
        String::from("scrimunban")
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        let command = CONFIG
            .guild
            .create_application_command(&ctx, |c| {
                c.name(self.name())
                    .description("Screenshare-unbans the given user.")
                    .default_permission(false)
                    .create_option(|o| {
                        o.name("user")
                            .description("The user to unban")
                            .required(true)
                            .kind(ApplicationCommandOptionType::User)
                    })
                    .create_option(|o| {
                        o.name("reason")
                            .description("The reason to remove this user's ban")
                            .kind(ApplicationCommandOptionType::String)
                            .required(false)
                    })
            })
            .await?;
        CONFIG
            .guild
            .create_application_command_permission(&ctx, command.id, |c| {
                c.create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.ss_support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
                        .id(CONFIG.support.0)
                        .permission(true)
                })
                .create_permission(|p| {
                    p.kind(ApplicationCommandPermissionType::Role)
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
        UnbanType::Scrim.exec(&ctx.http, command).await?;
        Ok(())
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
