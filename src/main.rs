use serenity::client::bridge::gateway::GatewayIntents;
use serenity::http::Http;
use serenity::Client;
use crate::consts::GUILD;
use crate::handler::Handler;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

mod commands;
mod handler;
mod consts;

// Bridge scrims guild id

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    tracing_subscriber::fmt().init();
    let application_id =
        Http::new_with_token(&std::env::var("BOT_TOKEN").expect("BOT_TOKEN not set"))
            .get_current_application_info()
            .await?
            .id
            .0;
    let mut client = Client::builder(&std::env::var("BOT_TOKEN")?)
        .application_id(application_id)
        .event_handler(Handler::new())
        .intents(GatewayIntents::GUILD_MESSAGES)
        .await?;
    let shard_manager = client.shard_manager.clone();
    let http = client.cache_and_http.http.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not listen for ctrl+c");
        tracing::info!("Shutting down");
        if let Ok(commands) = GUILD.get_application_commands(&http).await {
            for command in commands {
                if let Err(err) = GUILD.delete_application_command(&http, command.id).await {
                    tracing::error!("Could not delete '{}': {}", command.name, err);
                }
            }
        }
        shard_manager.lock().await.shutdown_all().await;
    });
    while let Err(err) = client.start().await {
        tracing::error!("{}", err);
    }
    Ok(())
}
