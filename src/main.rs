use serenity::model::gateway::GatewayIntents;
use serenity::Client;
use tracing_subscriber::filter::LevelFilter;

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
async fn main() {
    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(LevelFilter::INFO)
        .init();

    consts::DATABASE
        .init()
        .await
        .expect("Failed to initialize database");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_PRESENCES;

    let mut client = Client::builder(&SECRETS.bot_token, intents)
        .event_handler(Handler::new())
        .await
        .expect("Failed to connect to Discord!");

    while let Err(err) = client.start().await {
        tracing::error!("{}", err);
    }
}
