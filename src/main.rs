use crate::consts::CONFIG;
use crate::handler::Handler;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::http::Http;
use serenity::Client;
use tracing_subscriber::{
    filter::LevelFilter, fmt::Layer, layer::SubscriberExt, Layer as _, Registry,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

mod commands;
mod consts;
mod db;
mod handler;
#[macro_use]
mod macros;
mod model;

#[tokio::main]
async fn main() -> Result<()> {
    let file_appender = tracing_appender::rolling::daily(".", "logs.txt");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let filter = LevelFilter::INFO;
    let subscriber = Registry::default()
        .with(Layer::default().pretty().with_filter(filter.clone()))
        .with(
            Layer::default()
                .with_ansi(false)
                .with_writer(non_blocking)
                .with_filter(filter),
        );

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let application_id = Http::new_with_token(&CONFIG.bot_token)
        .get_current_application_info()
        .await?
        .id
        .0;
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(&CONFIG.bot_token)
        .application_id(application_id)
        .event_handler(Handler::new())
        .intents(intents)
        .await?;
    let shard_manager = client.shard_manager.clone();
    let http = client.cache_and_http.http.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not listen for ctrl+c");
        tracing::info!("Shutting down");
        tracing::info!("Deleting Guild Commands...");
        if let Ok(commands) = CONFIG.guild.get_application_commands(&http).await {
            for command in commands {
                if let Err(err) = CONFIG
                    .guild
                    .delete_application_command(&http, command.id)
                    .await
                {
                    tracing::error!("Could not delete guild command '{}': {}", command.name, err);
                }
            }
        }
        tracing::info!("Deleting Global Commands...");
        if let Ok(commands) = &http.get_global_application_commands().await {
            for command in commands {
                if let Err(err) = &http.delete_global_application_command(command.id.0).await {
                    tracing::error!(
                        "Could not delete global command '{}': {}",
                        command.name,
                        err
                    );
                }
            }
        }
        tracing::info!("Ending process...");
        shard_manager.lock().await.shutdown_all().await;
    });
    while let Err(err) = client.start().await {
        tracing::error!("{}", err);
    }
    Ok(())
}
