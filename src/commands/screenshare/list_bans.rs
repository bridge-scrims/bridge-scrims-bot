use std::fmt::Write;

use serenity::{
    async_trait, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::prelude::*,
};

use bridge_scrims::interaction::*;

pub struct ListBans;

#[async_trait]
impl InteractionHandler for ListBans {
    fn name(&self) -> String {
        String::from("list_bans")
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        Some(vec![
            crate::CONFIG.ss_support,
            crate::CONFIG.support,
            crate::CONFIG.trial_support,
        ])
    }

    async fn register(&self, ctx: &Context) -> crate::Result<()> {
        crate::CONFIG
            .guild
            .create_application_command(&ctx.http, |cmd| {
                cmd.name(self.name())
                    .description("List all of the scrim bans")
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
        let bans = crate::consts::DATABASE.fetch_scrim_unbans().await?;
        let mut desc = vec![String::new()];
        for ban in bans.into_iter().filter(|b| !b.is_expired()) {
            if desc[desc.len() - 1].len() > 1950 {
                desc.push(String::new())
            }
            let t = desc.len() - 1;
            writeln!(
                desc[t],
                "- <@!{}>: banned until <t:{}:R>",
                ban.user_id,
                ban.expires_at.unwrap().unix_timestamp()
            )?;
        }
        command
            .create_interaction_response(&ctx.http, |resp| {
                resp.interaction_response_data(|data| {
                    data.embed(|embed| embed.title("Scrim Bans").description(desc.pop().unwrap()))
                })
            })
            .await?;
        for d in desc {
            command
                .create_followup_message(&ctx.http, |resp| resp.embed(|embed| embed.description(d)))
                .await?;
        }
        Ok(None)
    }

    fn new() -> Box<Self> {
        Box::new(Self)
    }
}
