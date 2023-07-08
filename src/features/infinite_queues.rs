use regex::Regex;
use serenity::{
    model::{prelude::*, voice::VoiceState},
    prelude::*,
};
use std::time::Duration;

use crate::consts::CONFIG;
use crate::Result;

lazy_static::lazy_static! {
    pub static ref CHANNEL_GROUPS: Vec<ChannelGroup> = vec!(
        ChannelGroup(759949915681456209), ChannelGroup(903110641458491473), ChannelGroup(759950225111646208),
        ChannelGroup(850033618265047040), ChannelGroup(850034475579998268), ChannelGroup(905093037519175681),
        ChannelGroup(850034620983803914), ChannelGroup(954252151436242944), ChannelGroup(774001218275377162),
        ChannelGroup(774000992772947978), ChannelGroup(903749931033034784), ChannelGroup(774001398907404348),
        ChannelGroup(1063105974925279352), ChannelGroup(840697273175244881), ChannelGroup(760201083234156626),
        ChannelGroup(903749778154876948), ChannelGroup(760202194745819207), ChannelGroup(940024195553853540),
        ChannelGroup(759950829309919252), ChannelGroup(759950758652805190), ChannelGroup(903110991603191808),
        ChannelGroup(759951447001137184)
    );
    pub static ref NAME_REGEX: Regex = Regex::new(r"(.* #?)(\d+)").unwrap();
    pub static ref MIN_CHANNELS: usize = 2;
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

pub struct ChannelGroup(pub u64);
impl ChannelGroup {
    fn get_channels(&self, ctx: &Context) -> Vec<GuildChannel> {
        let channels = ctx.cache.guild_channels(CONFIG.guild);
        if let Some(channels) = channels {
            let patient0 = ctx.cache.guild_channel(self.0);
            if let Some(patient0) = patient0 {
                let (base_name, _) = divide_channel_name(patient0.name());
                return channels
                    .into_iter()
                    .map(|v| v.1)
                    .filter(|v| v.kind == ChannelType::Voice)
                    .filter(|v| v.parent_id == patient0.parent_id)
                    .filter(|v| v.name().starts_with(base_name))
                    .collect::<Vec<_>>();
            }
        }
        Vec::new()
    }

    async fn remove_excess_channels(&self, ctx: &Context) -> Result<()> {
        let channels = self.get_channels(ctx);
        for channel in channels {
            let (_, num) = divide_channel_name(channel.name());
            let count = count_vc_members(ctx, &channel);
            if count == 0 && num > *MIN_CHANNELS {
                let _ = channel.delete(ctx).await?;
            }
        }
        Ok(())
    }

    fn needs_more(&self, ctx: &Context) -> bool {
        let channels = self.get_channels(ctx);
        let all_used = channels.iter().all(|vc| count_vc_members(ctx, vc) > 0);
        all_used
    }

    fn highest_num(&self, ctx: &Context) -> usize {
        self.get_channels(ctx)
            .iter()
            .map(|c| divide_channel_name(c.name()).1)
            .max()
            .unwrap_or(0)
    }

    fn highest_pos(&self, ctx: &Context) -> u32 {
        self.get_channels(ctx)
            .iter()
            .map(|c| c.position.try_into().unwrap_or(0))
            .max()
            .unwrap_or(0)
    }

    async fn make_clone(&self, ctx: &Context) -> Result<()> {
        let patient0 = ctx.cache.guild_channel(self.0);
        if let Some(patient0) = patient0 {
            let num = self.highest_num(ctx) + 1;
            let pos = self.highest_pos(ctx);
            let (base_name, _) = divide_channel_name(patient0.name());
            let _ = patient0
                .guild(ctx)
                .unwrap()
                .create_channel(ctx, |new| {
                    if let Some(category_id) = patient0.parent_id {
                        new.category(category_id);
                    }
                    if let Some(user_limit) = patient0.rate_limit_per_user {
                        new.user_limit(user_limit.try_into().unwrap());
                    }
                    new.kind(ChannelType::Voice)
                        .name(base_name.to_owned() + &num.to_string())
                        .permissions(patient0.permission_overwrites.clone())
                        .position(pos)
                })
                .await?;
        }
        Ok(())
    }
}

pub struct InfiniteQueues;
impl InfiniteQueues {
    pub async fn on_voice_update(ctx: &Context, old: Option<&VoiceState>, new: &VoiceState) {
        // user moved to a new vc
        if new.channel_id.is_some() && old.map_or(true, |old| old.channel_id != new.channel_id) {
            for group in CHANNEL_GROUPS.iter() {
                if group.needs_more(ctx) {
                    let res1 = group.make_clone(ctx).await;
                    let res2 = group.make_clone(ctx).await;
                    if let Err(err) = res1.and(res2) {
                        tracing::error!(err)
                    }
                }
            }
        }
    }

    pub fn init(ctx: &Context) {
        tokio::spawn(excess_channel_cleanup_loop(ctx.clone()));
    }
}

async fn excess_channel_cleanup_loop(ctx: Context) {
    loop {
        for group in CHANNEL_GROUPS.iter() {
            let res = group.remove_excess_channels(&ctx).await;
            if let Err(err) = res {
                tracing::error!(err)
            }
        }
        tokio::time::sleep(Duration::from_secs(15 * 60)).await;
    }
}
