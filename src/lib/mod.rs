pub mod cooldown;
pub mod discord_util;
pub mod interaction;
pub mod parse_durations;
pub mod print_embeds;

pub type Error = dyn std::error::Error + Send + Sync;
pub type Result<T> = std::result::Result<T, Box<Error>>;
