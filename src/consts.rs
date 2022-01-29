use std::path::PathBuf;

use crate::db::Database;
use crate::dotenv;
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;
use serenity::model::id::RoleId;

lazy_static::lazy_static! {
    // Bridge scrims guild id
    pub static ref GUILD: GuildId = GuildId(dotenv!("GUILD").parse().unwrap());

    // Council related
    pub static ref PRIME_COUNCIL: RoleId = RoleId(dotenv!("PRIME_COUNCIL").parse().unwrap());
    pub static ref PRIME_HEAD: RoleId = RoleId(dotenv!("PRIME_HEAD").parse().unwrap());
    pub static ref PRIVATE_COUNCIL: RoleId = RoleId(dotenv!("PRIVATE_COUNCIL").parse().unwrap());
    pub static ref PRIVATE_HEAD: RoleId = RoleId(dotenv!("PRIVATE_HEAD").parse().unwrap());
    pub static ref PREMIUM_COUNCIL: RoleId = RoleId(dotenv!("PREMIUM_COUNCIL").parse().unwrap());
    pub static ref PREMIUM_HEAD: RoleId = RoleId(dotenv!("PREMIUM_HEAD").parse().unwrap());


    // Ban and mute related
    pub static ref BANNED: RoleId = RoleId(dotenv!("BANNED").parse().unwrap());
    pub static ref SS_SUPPORT: RoleId = RoleId(dotenv!("SS_SUPPORT").parse().unwrap());
    pub static ref STAFF: RoleId = RoleId(dotenv!("STAFF").parse().unwrap());
    pub static ref SUPPORT: RoleId = RoleId(dotenv!("SUPPORT").parse().unwrap());
    pub static ref TRIAL_SUPPORT: RoleId = RoleId(dotenv!("TRIAL_SUPPORT").parse().unwrap());

    // Database related
    pub static ref DATABASE_PATH: PathBuf = dirs::cache_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    pub static ref DATABASE: Database = Database::init();

    // Channel ids
    pub static ref SUPPORT_BANS: ChannelId = ChannelId(dotenv!("SUPPORT_BANS").parse().unwrap());
}
