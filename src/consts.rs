use serenity::model::id::GuildId;
use serenity::model::id::RoleId;
lazy_static::lazy_static! {
    pub static ref GUILD: GuildId = GuildId(std::env::var("GUILD").unwrap().parse().unwrap());
    pub static ref PRIME_COUNCIL: RoleId = RoleId(std::env::var("PRIME_COUNCIL").unwrap().parse().unwrap());
    pub static ref PRIME_HEAD: RoleId = RoleId(std::env::var("PRIME_HEAD").unwrap().parse().unwrap());
    pub static ref PRIVATE_COUNCIL: RoleId = RoleId(std::env::var("PRIVATE_COUNCIL").unwrap().parse().unwrap());
    pub static ref PRIVATE_HEAD: RoleId = RoleId(std::env::var("PRIVATE_HEAD").unwrap().parse().unwrap());
    pub static ref PREMIUM_COUNCIL: RoleId = RoleId(std::env::var("PREMIUM_COUNCIL").unwrap().parse().unwrap());
    pub static ref PREMIUM_HEAD: RoleId = RoleId(std::env::var("PREMIUM_HEAD").unwrap().parse().unwrap());
}