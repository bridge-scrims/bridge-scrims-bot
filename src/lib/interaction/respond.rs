use serenity::{
    async_trait,
    builder::{CreateInteractionResponse, CreateInteractionResponseData},
    http::Http,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction,
        message_component::MessageComponentInteraction, MessageFlags,
    },
    model::prelude::*,
    Result,
};

#[async_trait]
pub trait RespondableInteraction {
    async fn respond<'a>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        resp: CreateInteractionResponseData<'a>,
    ) -> Result<()>;
    async fn edit_response<'a>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        resp: CreateInteractionResponseData<'a>,
    ) -> Result<()>;
    async fn create_response<'a>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        resp: CreateInteractionResponse<'a>,
    ) -> Result<()>;
}

#[async_trait]
impl RespondableInteraction for ApplicationCommandInteraction {
    async fn create_response<'a>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        resp: CreateInteractionResponse<'a>,
    ) -> Result<()> {
        self.create_interaction_response(http.as_ref(), |d| {
            *d = resp;
            d
        })
        .await?;
        Ok(())
    }

    async fn respond<'a>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        resp: CreateInteractionResponseData<'a>,
    ) -> Result<()> {
        self.create_interaction_response(http.as_ref(), |d| {
            d.interaction_response_data(|d| {
                d.0 = resp.0;
                d.1 = resp.1;
                if !d.0.contains_key("flags") {
                    d.flags(MessageFlags::EPHEMERAL);
                }
                d
            });
            d
        })
        .await?;
        Ok(())
    }

    async fn edit_response<'a>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        resp: CreateInteractionResponseData<'a>,
    ) -> Result<()> {
        self.edit_original_interaction_response(http.as_ref(), |d| {
            d.0 = resp.0;
            d
        })
        .await?;
        Ok(())
    }
}

#[async_trait]
impl RespondableInteraction for MessageComponentInteraction {
    async fn create_response<'a>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        resp: CreateInteractionResponse<'a>,
    ) -> Result<()> {
        self.create_interaction_response(http.as_ref(), |d| {
            *d = resp;
            d
        })
        .await?;
        Ok(())
    }

    async fn respond<'a>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        resp: CreateInteractionResponseData<'a>,
    ) -> Result<()> {
        self.create_interaction_response(http.as_ref(), |d| {
            if self
                .message
                .flags
                .unwrap_or_default()
                .contains(channel::MessageFlags::EPHEMERAL)
            {
                d.kind(interaction::InteractionResponseType::UpdateMessage);
            }
            d.interaction_response_data(|d| {
                d.0 = resp.0;
                d.1 = resp.1;
                if !d.0.contains_key("flags") {
                    d.flags(MessageFlags::EPHEMERAL);
                }
                d
            });
            d
        })
        .await?;
        Ok(())
    }

    async fn edit_response<'a>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        resp: CreateInteractionResponseData<'a>,
    ) -> Result<()> {
        self.edit_original_interaction_response(http.as_ref(), |d| {
            d.0 = resp.0;
            d
        })
        .await?;
        Ok(())
    }
}
