use std::path::PathBuf;

use crate::db::Database;
use serde::Deserialize;
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;
use serenity::model::id::RoleId;
use std::collections::HashMap;
use std::fs;
use toml::from_str;

#[derive(Deserialize)]
pub struct Config {
    pub bot_token: String,

    pub guild: GuildId,

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

    pub polls: ChannelId,
    pub clips: ChannelId,

    pub prefabs: HashMap<String, String>,
}

lazy_static::lazy_static! {
    // Database related
    pub static ref DATABASE_PATH: PathBuf = dirs::cache_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    pub static ref DATABASE: Database = Database::init();

    pub static ref CONFIG_STRING: String = fs::read_to_string("config.toml").expect("Config Not Supplied");

    pub static ref CONFIG: Config = from_str(&CONFIG_STRING).expect("Config could not be parsed.");
}
