use serenity::model::id::UserId;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tokio::time::sleep;

#[derive(Clone, Copy, PartialEq)]
pub enum CooldownType {
    User(UserId),
    Global,
}

#[derive(Clone, PartialEq)]
pub struct Cooldown {
    duration: Duration,
    expire: SystemTime,
    key: Option<String>,
    cooldown_info: CooldownType,
}

impl Cooldown {
    pub fn new(duration: Duration, key: Option<String>, cooldown_info: CooldownType) -> Cooldown {
        Cooldown {
            duration,
            expire: SystemTime::now() + duration,
            key,
            cooldown_info,
        }
    }
}

#[derive(Default, Clone)]
pub struct Cooldowns(Arc<Mutex<Vec<Cooldown>>>);

impl Cooldowns {
    pub fn new() -> Self {
        Self::default()
    }
    async fn remove_cooldown(inner: Arc<Mutex<Vec<Cooldown>>>, cooldown: Cooldown) {
        sleep(cooldown.duration).await;
        let mut c = inner.lock().await;
        if let Some(i) = c.iter().position(|v| v == &cooldown) {
            c.swap_remove(i);
        }
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
        tokio::spawn(Cooldowns::remove_cooldown(self.0.clone(), cooldown));
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
        self.0.lock().await.iter().find(|cooldown| {
            if key != cooldown.key && cooldown.key.is_some() {
                return false;
            };
            match cooldown.cooldown_info {
                CooldownType::Global => true,
                CooldownType::User(uid) => uid == user,
            }
        })
        .map(|cooldown| {
            cooldown.duration - (cooldown.expire - cooldown.duration).elapsed().unwrap()
        })
    }

    pub async fn check_cooldown(&self, user: UserId) -> Option<Duration> {
        self.has_cooldown(None, user).await
    }
    pub async fn check_cooldown_key(&self, user: UserId, key: String) -> Option<Duration> {
        self.has_cooldown(Some(key), user).await
    }
}
