use serenity::async_trait;
use serenity::client::Context;
use serenity::model::prelude::application_command::ApplicationCommandInteraction;

pub mod council;
pub mod prefabs;
pub mod timeout;

#[async_trait]
pub trait Command {
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
