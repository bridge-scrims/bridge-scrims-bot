use std::{collections::HashMap, env, fs, path::PathBuf};

use serde::Deserialize;
use serenity::{client::Context, model::prelude::*};
use toml::from_str;

use crate::db::Database;

#[derive(Deserialize)]
pub struct MemberCount(ChannelId);

impl MemberCount {
    pub async fn update(&self, ctx: &Context, guild_id: GuildId) -> crate::Result<()> {
        let guild = guild_id.to_guild_cached(&ctx.cache).unwrap();
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
    pub required_roles: Vec<RoleId>,
    pub options: HashMap<String, RoleId>,
    pub allowed_channels: Option<Vec<ChannelId>>,
}

#[derive(Deserialize, Clone)]
pub struct Council {
    pub head: RoleId,
    pub role: RoleId,
}

#[derive(Deserialize)]
pub struct Config {
    pub guild: GuildId,

    pub queue_categories: Vec<ChannelId>,

    pub member_role: RoleId,
    pub unverified_role: RoleId,

    pub councils: HashMap<String, Council>,

    pub banned: RoleId,
    pub ss_support: RoleId,
    pub head_of_ss: RoleId,
    pub staff: RoleId,
    pub support: RoleId,
    pub trial_support: RoleId,
    pub support_bans: ChannelId,
    pub screenshare_requests: ChannelId,
    pub muted_role: RoleId,
    pub frozen: RoleId,
    pub frozen_chat: ChannelId,
    pub hello_cheaters: ChannelId,
    pub ss_logs: ChannelId,
    pub freeze_emoji: String,
    pub unfreeze_emoji: String,

    pub upvote_downvote_channels: Vec<ChannelId>,
    pub like_react_channels: Vec<ChannelId>,

    pub clips: ChannelId,
    pub reaction_logs: ChannelId,
    pub color_roles: Vec<RoleId>,

    pub appeal_channel: ChannelId,
    pub appeal_forum: String,

    pub prefabs: HashMap<String, String>,

    pub member_count: MemberCount,

    pub pings: Vec<Ping>,
    pub q_and_a_channel: ChannelId,
    pub q_and_a_role: RoleId,

    pub booster_info: ChannelId,

    pub expanding_channels: Vec<ChannelId>,
    pub expanding_min: usize,
    pub expanding_max: usize,
}

pub struct Secrets {
    pub bot_token: String,
}

lazy_static::lazy_static! {
    pub static ref DATABASE_PATH: PathBuf = std::env::current_dir().unwrap();
    pub static ref DATABASE: Database = Database::init();

    pub static ref CONFIG: Config = {
        let config_string: String = fs::read_to_string("Config.toml").expect("Config Not Supplied!");
        from_str(&config_string).expect("Config could not be parsed!")
    };

    pub static ref SECRETS: Secrets = Secrets {
        bot_token: env::var("BOT_TOKEN").unwrap(),
    };
}
