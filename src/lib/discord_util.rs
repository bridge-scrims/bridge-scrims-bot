use serenity::{
    model::prelude::{GuildChannel, UserId},
    prelude::Context,
};

pub fn vc_members(ctx: &Context, vc: &GuildChannel) -> Vec<UserId> {
    vc.guild(ctx)
        .unwrap()
        .voice_states
        .values()
        .filter(|v| v.channel_id == Some(vc.id))
        .map(|v| v.user_id)
        .collect()
}
