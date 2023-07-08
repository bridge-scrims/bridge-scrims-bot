use serenity::http::Http;
use serenity::model::gateway::GatewayIntents;
use serenity::Client;
use tracing_subscriber::{
    filter::LevelFilter, fmt::Layer, layer::SubscriberExt, Layer as _, Registry,
};

use crate::consts::{CONFIG, SECRETS};
use crate::handler::Handler;
use bridge_scrims::Result;

mod commands;
mod consts;
mod db;
mod features;
mod handler;
#[macro_use]
mod macros;
mod model;

#[tokio::main]
async fn main() -> Result<()> {
    let file_appender = tracing_appender::rolling::daily("logs", "rolling.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let filter = LevelFilter::INFO;
    let subscriber = Registry::default()
        .with(Layer::default().pretty().with_filter(filter))
        .with(
            Layer::default()
                .pretty()
                .with_ansi(false)
                .compact()
                .with_writer(non_blocking)
                .with_filter(filter),
        );

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let application_id = Http::new(&SECRETS.bot_token)
        .get_current_application_info()
        .await?
        .id
        .0;
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_PRESENCES;

    let mut client = Client::builder(&SECRETS.bot_token, intents)
        .application_id(application_id)
        .event_handler(Handler::new())
        .await?;
    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not listen for ctrl+c");
        tracing::info!("Shutting down");
        tracing::info!("Ending process...");
        shard_manager.lock().await.shutdown_all().await;
    });
    while let Err(err) = client.start().await {
        tracing::error!("{}", err);
    }
    Ok(())
}
