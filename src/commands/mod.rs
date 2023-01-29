use serenity::async_trait;
use serenity::client::Context;
use serenity::model::application::interaction::{
    application_command::ApplicationCommandInteraction,
    message_component::MessageComponentInteraction,
};

pub mod ban;
pub mod close;
pub mod council;
pub mod freeze;
pub mod list_bans;
pub mod logtime;
pub mod notes;
pub mod ping;
pub mod prefabs;
pub mod purge;
pub mod reaction;
pub mod reload;
pub mod teams;
pub mod screenshare;
pub mod screensharers;
pub mod ticket;
pub mod timeout;
pub mod unban;
pub mod unfreeze;

#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> String;
    async fn init(&self, _ctx: &Context) {
        // init will only be executed once.
    }
    async fn register(&self, ctx: &Context) -> crate::Result<()>;
    // will be exectued on reloads
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
