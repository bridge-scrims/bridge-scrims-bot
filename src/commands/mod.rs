use serenity::async_trait;
use serenity::client::Context;
use serenity::model::id::UserId;
use serenity::model::prelude::application_command::ApplicationCommandInteraction;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tokio::time::sleep;
pub mod ban;
pub mod council;
pub mod notes;
pub mod ping;
pub mod prefabs;
pub mod purge;
pub mod reaction;
pub mod roll;
pub mod timeout;

#[async_trait]
pub trait Command {
    fn name(&self) -> String;
    async fn register(&self, ctx: &Context) -> crate::Result<()>;
    async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> crate::Result<()>;
    fn new() -> Box<Self>
    where
        Self: Sized;
}
#[derive(Clone, Copy, PartialEq)]
pub enum CooldownType {
    User(UserId),
    Global,
}

#[derive(Clone, PartialEq)]
pub struct Cooldown {
    duration: Duration,
    exipre: SystemTime,
    key: Option<String>,
    cooldown_info: CooldownType,
}

impl Cooldown {
    pub fn new(duration: Duration, key: Option<String>, cooldown_info: CooldownType) -> Cooldown {
        Cooldown {
            duration,
            exipre: SystemTime::now() + duration,
            key,
            cooldown_info,
        }
    }
}

pub struct Cooldowns(Arc<Mutex<Vec<Cooldown>>>);

impl Cooldowns {
    pub fn new() -> Cooldowns {
        Cooldowns(Arc::new(Mutex::new(Vec::new())))
    }
    async fn remove_cooldown(inner: Arc<Mutex<Vec<Cooldown>>>, cooldown: Cooldown) {
        sleep(cooldown.duration).await;
        let mut c = inner.lock().await;
        let mut x = (&*c).clone();
        for (i, v) in x.iter().enumerate() {
            if v == &cooldown {
                x.remove(i);
                break;
            }
        }
        *c = x
    }

    async fn add_cooldown(
        &self,
        duration: Duration,
        key: Option<String>,
        cooldown_type: CooldownType,
    ) {
        let mut c = self.0.lock().await;
        let cooldown = Cooldown::new(duration, key, cooldown_type);
        (*c).push(cooldown.clone());
        drop(c);
        // Make the cooldown automatically expire in the time
        tokio::spawn(Cooldowns::remove_cooldown(self.0.clone(), cooldown.clone()));
    }
    pub async fn add_global_cooldown(&self, duration: Duration) {
        self.add_cooldown(duration, None, CooldownType::Global)
            .await
    }
    pub async fn add_user_cooldown(&self, duration: Duration, user: UserId) {
        self.add_cooldown(duration, None, CooldownType::User(user))
            .await
    }
    pub async fn add_global_cooldown_key(&self, key: String, duration: Duration) {
        self.add_cooldown(duration, Some(key), CooldownType::Global)
            .await
    }
    pub async fn add_user_cooldown_key(&self, key: String, duration: Duration, user: UserId) {
        self.add_cooldown(duration, Some(key), CooldownType::User(user))
            .await
    }

    async fn has_cooldown(&self, key: Option<String>, user: UserId) -> Option<Duration> {
        let c = self.0.lock().await;
        for cooldown in &*c {
            if key != cooldown.key && cooldown.key.is_some() {
                continue;
            }
            let x = match cooldown.cooldown_info {
                CooldownType::Global => true,
                CooldownType::User(uid) => uid == user,
            };
            if x {
                return Some(
                    cooldown.duration - (cooldown.exipre - cooldown.duration).elapsed().unwrap(),
                );
            }
        }
        None
    }

    pub async fn check_cooldown(&self, user: UserId) -> Option<Duration> {
        self.has_cooldown(None, user).await
    }
    pub async fn check_cooldown_key(&self, user: UserId, key: String) -> Option<Duration> {
        self.has_cooldown(Some(key), user).await
    }
}
