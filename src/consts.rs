use serenity::model::id::GuildId;
lazy_static::lazy_static! {
    pub static ref GUILD: GuildId = GuildId(std::env::var("GUILD").unwrap().parse().unwrap());
    pub static ref PRIME_COUNCIL: GuildId = GuildId(std::env::var("PRIME_COUNCIL").unwrap().parse().unwrap());
    pub static ref PRIME_HEAD: GuildId = GuildId(std::env::var("PRIME_HEAD").unwrap().parse().unwrap());
    pub static ref PRIVATE_COUNCIL: GuildId = GuildId(std::env::var("PRIVATE_COUNCIL").unwrap().parse().unwrap());
    pub static ref PRIVATE_HEAD: GuildId = GuildId(std::env::var("PRIVATE_HEAD").unwrap().parse().unwrap());
    pub static ref PREMIUM_COUNCIL: GuildId = GuildId(std::env::var("PREMIUM_COUNCIL").unwrap().parse().unwrap());
    pub static ref PREMIUM_HEAD: GuildId = GuildId(std::env::var("PREMIUM_HEAD").unwrap().parse().unwrap());
}