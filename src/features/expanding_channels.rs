use futures::future::join_all;
use regex::Regex;
use serenity::{
    model::{prelude::*, voice::VoiceState},
    prelude::*,
};
use std::time::Duration;

use crate::consts::CONFIG;
use crate::Result;

lazy_static::lazy_static! {
    pub static ref NAME_REGEX: Regex = Regex::new(r"(.* #?)(\d+)").unwrap();
    pub static ref CHANNEL_GROUPS: Vec<Mutex<ChannelFamily>> = CONFIG.expanding_channels.iter().map(|patient0| Mutex::new(ChannelFamily(*patient0))).collect();
    pub static ref MIN_CHANNELS: usize = CONFIG.expanding_min;
    pub static ref MAX_CHANNELS: usize = CONFIG.expanding_max;
}

fn divide_channel_name(name: &str) -> (&str, usize) {
    let captures = NAME_REGEX.captures(name);
    let base_name = captures
        .as_ref()
        .map_or(name, |m| m.get(1).unwrap().as_str());
    let index = captures.map_or(1, |m| m.get(2).unwrap().as_str().parse::<usize>().unwrap());
    (base_name, index)
}

fn count_vc_members(ctx: &Context, vc: &GuildChannel) -> usize {
    vc.guild(ctx)
        .unwrap()
        .voice_states
        .values()
        .filter(|v| v.channel_id == Some(vc.id))
        .count()
}

pub struct ChannelFamily(pub ChannelId);
impl ChannelFamily {
    fn get_channels(&self, ctx: &Context) -> Vec<GuildChannel> {
        let channels = ctx.cache.guild_channels(CONFIG.guild);
        if let Some(channels) = channels {
            let patient0 = ctx.cache.guild_channel(self.0);
            if let Some(patient0) = patient0 {
                let (base_name, _) = divide_channel_name(patient0.name());
                let mut channels = channels
                    .into_iter()
                    .map(|v| v.1)
                    .filter(|v| v.kind == ChannelType::Voice)
                    .filter(|v| v.parent_id == patient0.parent_id)
                    .filter(|v| v.name().starts_with(base_name))
                    .collect::<Vec<_>>();

                channels.sort_by_key(|c| divide_channel_name(c.name()).1);
                return channels;
            }
        }
        Vec::new()
    }

    async fn remove_excess_channels(&self, ctx: &Context) -> Result<()> {
        let channels = self.get_channels(ctx);
        let not_used = channels
            .iter()
            .filter(|vc| count_vc_members(ctx, vc) == 0)
            .collect::<Vec<_>>();

        let remove_count = not_used.len();
        if remove_count > *MIN_CHANNELS {
            let remove = remove_count - *MIN_CHANNELS;
            for channel in not_used.iter().rev().take(remove) {
                let _ = channel.delete(ctx).await?;
            }
        }
        Ok(())
    }

    fn needs_more(&self, ctx: &Context) -> bool {
        let channels = self.get_channels(ctx);
        let all_used = channels.iter().all(|vc| count_vc_members(ctx, vc) > 0);
        all_used && channels.len() < *MAX_CHANNELS
    }

    fn next_num_pos(&self, ctx: &Context) -> (usize, usize) {
        let highest_num = self
            .get_channels(ctx)
            .iter()
            .map(|c| divide_channel_name(c.name()).1)
            .max()
            .unwrap_or(1);

        let highest_pos = self
            .get_channels(ctx)
            .iter()
            .map(|c| usize::try_from(c.position).unwrap_or(0))
            .max()
            .unwrap_or(0);

        (highest_num + 1, highest_pos)
    }

    async fn make_clone(&self, ctx: &Context) -> Result<()> {
        let patient0 = ctx.cache.guild_channel(self.0);
        if let Some(patient0) = patient0 {
            let (num, pos) = self.next_num_pos(ctx);
            let (base_name, _) = divide_channel_name(patient0.name());
            let _ = patient0
                .guild(ctx)
                .unwrap()
                .create_channel(ctx, |new| {
                    if let Some(category_id) = patient0.parent_id {
                        new.category(category_id);
                    }
                    if let Some(user_limit) = patient0.user_limit {
                        new.user_limit(user_limit.try_into().unwrap());
                    }
                    new.kind(ChannelType::Voice)
                        .name(base_name.to_owned() + &num.to_string())
                        .permissions(patient0.permission_overwrites.clone())
                        .position(pos.try_into().unwrap())
                })
                .await?;
        }
        Ok(())
    }
}

pub struct ExpandingChannels;
impl ExpandingChannels {
    pub async fn on_voice_update(ctx: &Context, old: Option<&VoiceState>, new: &VoiceState) {
        // user either moved to a new vc or joined a vc
        if new.channel_id.is_some() && old.map_or(true, |old| old.channel_id != new.channel_id) {
            join_all(CHANNEL_GROUPS.iter().map(|group| async {
                let locked_group = group.lock().await;
                if locked_group.needs_more(ctx) {
                    let res1 = locked_group.make_clone(ctx).await;
                    let res2 = locked_group.make_clone(ctx).await;
                    if let Err(err) = res1.and(res2) {
                        tracing::error!("Error cloning channel: {}", err)
                    }
                }
            }))
            .await;
        }
    }

    pub fn init(ctx: &Context) {
        tokio::spawn(excess_channel_cleanup_loop(ctx.clone()));
    }
}

async fn excess_channel_cleanup_loop(ctx: Context) {
    loop {
        join_all(CHANNEL_GROUPS.iter().map(|group| async {
            let locked_group = group.lock().await;
            let res = locked_group.remove_excess_channels(&ctx).await;
            if let Err(err) = res {
                tracing::error!("Error deleting channel: {}", err)
            }
        }))
        .await;
        tokio::time::sleep(Duration::from_secs(15 * 60)).await;
    }
}
