use serenity::async_trait;
use serenity::client::Context;
use serenity::model::interactions::message_component::MessageComponentInteraction;
use serenity::model::prelude::application_command::ApplicationCommandInteraction;
pub mod ban;
pub mod council;
pub mod notes;
pub mod ping;
pub mod prefabs;
pub mod purge;
pub mod reaction;
pub mod roll;
pub mod timeout;
pub mod screenshare;
pub mod close;
pub mod freeze;
pub mod unfreeze;
pub mod ticket;

#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> String;
    async fn register(&self, ctx: &Context) -> crate::Result<()>;
    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()>;
    fn new() -> Box<Self>
    where
        Self: Sized;
}

#[async_trait]
pub trait Button: Command {
    async fn click(
        &self,
        ctx: &Context,
        command: &MessageComponentInteraction,
    ) -> crate::Result<()>;
}
