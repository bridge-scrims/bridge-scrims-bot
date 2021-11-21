use crate::dotenv;
use serenity::model::id::GuildId;
use serenity::model::id::RoleId;

lazy_static::lazy_static! {
    // Bridge scrims guild id
    pub static ref GUILD: GuildId = GuildId(dotenv!("GUILD").parse().unwrap());

    pub static ref BANNED: RoleId = RoleId(dotenv!("BANNED").parse().unwrap());
    pub static ref PRIME_COUNCIL: RoleId = RoleId(dotenv!("PRIME_COUNCIL").parse().unwrap());
    pub static ref PRIME_HEAD: RoleId = RoleId(dotenv!("PRIME_HEAD").parse().unwrap());
    pub static ref PRIVATE_COUNCIL: RoleId = RoleId(dotenv!("PRIVATE_COUNCIL").parse().unwrap());
    pub static ref PRIVATE_HEAD: RoleId = RoleId(dotenv!("PRIVATE_HEAD").parse().unwrap());
    pub static ref PREMIUM_COUNCIL: RoleId = RoleId(dotenv!("PREMIUM_COUNCIL").parse().unwrap());
    pub static ref PREMIUM_HEAD: RoleId = RoleId(dotenv!("PREMIUM_COUNCIL").parse().unwrap());
    pub static ref SS_SUPPORT: RoleId = RoleId(dotenv!("SS_SUPPORT").parse().unwrap());
}
