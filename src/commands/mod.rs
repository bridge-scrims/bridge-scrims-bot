use serenity::async_trait;
use serenity::client::Context;
use serenity::model::interactions::message_component::MessageComponentInteraction;
use serenity::model::prelude::application_command::ApplicationCommandInteraction;
pub mod ban;
pub mod close;
pub mod council;
pub mod freeze;
pub mod list_bans;
pub mod notes;
pub mod ping;
pub mod prefabs;
pub mod purge;
pub mod reaction;
pub mod reload;
pub mod roll;
pub mod screenshare;
pub mod screensharers;
pub mod ticket;
pub mod timeout;
pub mod unban;
pub mod unfreeze;

#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> String;
    async fn register(&self, ctx: &Context) -> crate::Result<()>;
    fn is_command(&self, name: String) -> bool {
        self.name() == name
    }
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
