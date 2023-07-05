use futures::FutureExt;
use std::panic::AssertUnwindSafe;

use serenity::{
    async_trait,
    builder::{CreateInteractionResponse, CreateInteractionResponseData},
    client::Context,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction,
        message_component::MessageComponentInteraction, MessageFlags,
    },
    model::prelude::*,
};

use super::err_resp::ErrorResponse;
use super::respond::RespondableInteraction;

#[allow(dead_code)]
pub enum InitialInteractionResponse {
    DeferEphemeralReply,
    DeferReply,
    DeferUpdate,
    None,
}

pub type InteractionResult<'a> = crate::Result<Option<CreateInteractionResponseData<'a>>>;

#[async_trait]
pub trait InteractionHandler: Send + Sync {
    async fn init(&self, _ctx: &Context) {
        // init will only be executed once on bot start up
    }

    async fn register(&self, _ctx: &Context) -> crate::Result<()> {
        Ok(())
    }

    fn name(&self) -> String;
    fn is_handler(&self, name: String) -> bool {
        self.name() == name
    }

    fn initial_response(
        &self,
        _interaction_type: interaction::InteractionType,
    ) -> InitialInteractionResponse {
        InitialInteractionResponse::None
    }

    fn get_initial_response(
        &self,
        interaction_type: interaction::InteractionType,
    ) -> Option<CreateInteractionResponse> {
        let mut resp = CreateInteractionResponse::default();
        resp.kind(interaction::InteractionResponseType::DeferredChannelMessageWithSource);
        match self.initial_response(interaction_type) {
            InitialInteractionResponse::DeferEphemeralReply => {
                resp.interaction_response_data(|d| d.flags(MessageFlags::EPHEMERAL));
            }
            InitialInteractionResponse::DeferUpdate => {
                resp.kind(interaction::InteractionResponseType::DeferredUpdateMessage);
            }
            InitialInteractionResponse::DeferReply => (),
            InitialInteractionResponse::None => return None,
        }
        Some(resp)
    }

    fn no_permissions_error(&self) -> crate::Result<()> {
        Err(ErrorResponse::with_title(
            "Insufficient Permissions",
            "You are missing the required permissions to run this command!",
        ))?
    }

    fn unexpected_error<'a>(&self) -> Box<ErrorResponse<'a>> {
        ErrorResponse::with_footer(
            "Whoopsie!", 
            "\
                Unfortunately your command could not be handled due to something unexpected going wrong. \
                Sorry for the inconvenience. Maybe try again in a minute.\
            ",
            "If this issue persists, please report this to the developers."
        )
    }

    async fn verify_execution(
        &self,
        ctx: &Context,
        _user: &User,
        member: &Option<Member>,
        _channel: &ChannelId,
    ) -> crate::Result<()> {
        if member.is_none() {
            return self.no_permissions_error();
        }

        if let Ok(perms) = member.as_ref().unwrap().permissions(ctx) {
            if perms.administrator() {
                return Ok(());
            }
        }

        if let Some(allowed_roles) = self.allowed_roles() {
            if !member
                .as_ref()
                .unwrap()
                .roles(ctx)
                .unwrap_or_default()
                .iter()
                .any(|r| allowed_roles.contains(&r.id))
            {
                return self.no_permissions_error();
            }
        }

        Ok(())
    }

    fn allowed_roles(&self) -> Option<Vec<RoleId>> {
        None
    }

    async fn on_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()> {
        let initial_response = self.get_initial_response(command.kind);
        if let Some(initial_response) = initial_response.clone() {
            command.create_response(ctx, initial_response).await?;
        }

        let res = self._on_command(ctx, command).await;
        let resp = match res.as_ref() {
            Ok(resp) => resp.clone(),
            Err(err) => match err.downcast_ref::<Box<ErrorResponse>>() {
                Some(err) => Some(err.0.clone()),
                None => Some(self.unexpected_error().0),
            },
        };

        if let Some(resp) = resp {
            let _ = match initial_response {
                Some(_) => command.edit_response(ctx, resp).await,
                None => command.respond(ctx, resp).await,
            }
            .map_err(|err| tracing::error!("Sending InteractionErrorResponse failed: {}", err));
        }

        if let Err(err) = res {
            if err.downcast_ref::<Box<ErrorResponse>>().is_none() {
                return Err(err);
            }
        }

        Ok(())
    }

    async fn _on_command(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        self.verify_execution(ctx, &command.user, &command.member, &command.channel_id)
            .await?;
        let res = AssertUnwindSafe(self.handle_command(ctx, command))
            .catch_unwind()
            .await;
        match res {
            Err(_) => Err(self.unexpected_error())?, // on panic
            Ok(v) => v,
        }
    }

    async fn handle_command(
        &self,
        _ctx: &Context,
        _command: &ApplicationCommandInteraction,
    ) -> InteractionResult {
        Ok(None)
    }

    async fn on_component(
        &self,
        ctx: &Context,
        command: &MessageComponentInteraction,
        args: &[&str],
    ) -> crate::Result<()> {
        let initial_response = self.get_initial_response(command.kind);
        if let Some(initial_response) = initial_response.clone() {
            command.create_response(ctx, initial_response).await?;
        }

        let res = self._on_component(ctx, command, args).await;
        let resp = match res.as_ref() {
            Ok(resp) => resp.clone(),
            Err(err) => match err.downcast_ref::<Box<ErrorResponse>>() {
                Some(err) => Some(err.0.clone()),
                None => Some(self.unexpected_error().0),
            },
        };

        if let Some(resp) = resp {
            let _ = match initial_response {
                Some(_) => command.edit_response(ctx, resp).await,
                None => command.respond(ctx, resp).await,
            }
            .map_err(|err| tracing::error!("Sending InteractionErrorResponse failed: {}", err));
        }

        if let Err(err) = res {
            if err.downcast_ref::<Box<ErrorResponse>>().is_none() {
                return Err(err);
            }
        }

        Ok(())
    }

    async fn _on_component(
        &self,
        ctx: &Context,
        command: &MessageComponentInteraction,
        args: &[&str],
    ) -> InteractionResult {
        self.verify_execution(ctx, &command.user, &command.member, &command.channel_id)
            .await?;
        let res = AssertUnwindSafe(self.handle_component(ctx, command, args))
            .catch_unwind()
            .await;
        match res {
            Err(_) => Err(self.unexpected_error())?, // on panic
            Ok(v) => v,
        }
    }

    async fn handle_component(
        &self,
        _ctx: &Context,
        _command: &MessageComponentInteraction,
        _args: &[&str],
    ) -> InteractionResult {
        Ok(None)
    }

    fn new() -> Box<Self>
    where
        Self: Sized;
}
