use std::path::PathBuf;

use crate::db::Database;
use bridge_scrims::hypixel::UUID;
use serde::Deserialize;
use serenity::model::id::ChannelId;
use serenity::model::id::EmojiId;
use serenity::model::id::GuildId;
use serenity::model::id::RoleId;
use serenity::prelude::Context;
use std::collections::HashMap;
use std::fs;
use toml::from_str;

#[derive(Deserialize)]
pub struct MemberCount(ChannelId);

impl MemberCount {
    pub async fn update(&self, ctx: Context, guild_id: GuildId) -> crate::Result<()> {
        let guild = guild_id.to_guild_cached(&ctx.cache).await.unwrap();
        self.0
            .edit(&ctx.http, |c| {
                c.name(format!("Members: {}", guild.member_count))
            })
            .await?;
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct Ping {
    pub name: String,
    pub required_role: RoleId,
    pub options: HashMap<String, RoleId>,
}

#[derive(Deserialize)]
pub struct Config {
    pub bot_token: String,
    #[serde(deserialize_with = "bridge_scrims::hypixel::deserialize_uuid")]
    pub hypixel_token: UUID,

    pub guild: GuildId,

    pub queue_categories: Vec<ChannelId>,

    pub prime_council: RoleId,
    pub prime_head: RoleId,
    pub private_council: RoleId,
    pub private_head: RoleId,
    pub premium_council: RoleId,
    pub premium_head: RoleId,

    pub banned: RoleId,
    pub ss_support: RoleId,
    pub staff: RoleId,
    pub support: RoleId,
    pub trial_support: RoleId,
    pub support_bans: ChannelId,
    pub screenshare_requests: ChannelId,
    pub frozen: RoleId,
    pub frozen_chat: ChannelId,
    pub hello_cheaters: ChannelId,
    pub ss_logs: ChannelId,
    pub freeze_emoji: EmojiId,
    pub unfreeze_emoji: EmojiId,

    pub polls: ChannelId,
    pub clips: ChannelId,

    pub prefabs: HashMap<String, String>,

    pub member_count: MemberCount,

    pub pings: Vec<Ping>,
}

lazy_static::lazy_static! {
    // Database related
    pub static ref DATABASE_PATH: PathBuf = dirs::cache_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    pub static ref DATABASE: Database = Database::init();

    pub static ref CONFIG_STRING: String = fs::read_to_string("config.toml").expect("Config Not Supplied");

    pub static ref CONFIG: Config = from_str(&CONFIG_STRING).expect("Config could not be parsed.");
}
