use std::sync::OnceLock;

use sqlx::{postgres::PgPoolOptions, query, query_as, PgPool};
use time::OffsetDateTime;

pub use crate::model::*;
use crate::Result;

pub struct Database(pub OnceLock<PgPool>);

impl Database {
    pub const fn new() -> Database {
        Self(OnceLock::new())
    }
    pub async fn init(&self) -> Result<()> {
        let pool = PgPoolOptions::new()
            .connect(&std::env::var("DATABASE_URL")?)
            .await?;
        sqlx::migrate!().run(&pool).await?;
        self.0.set(pool).unwrap();
        Ok(())
    }

    pub fn get(&self) -> PgPool {
        self.0.get().unwrap().clone()
    }

    pub async fn fetch_scrim_unbans(&self) -> Result<Vec<ScrimUnban>> {
        Ok(query_as!(ScrimUnban, "SELECT * FROM scheduled_scrim_unban")
            .fetch_all(&self.get())
            .await?)
    }

    pub async fn fetch_custom_reactions(&self) -> Result<Vec<CustomReaction>> {
        Ok(query_as!(CustomReaction, "SELECT * FROM reaction")
            .fetch_all(&self.get())
            .await?)
    }

    pub async fn fetch_custom_reactions_for(&self, userid: u64) -> Result<Vec<CustomReaction>> {
        Ok(query_as!(
            CustomReaction,
            "SELECT * FROM reaction WHERE user_id = $1",
            userid as i64
        )
        .fetch_all(&self.get())
        .await?)
    }

    pub async fn fetch_custom_reactions_with_trigger(
        &self,
        trigger: &str,
    ) -> Result<Vec<CustomReaction>> {
        Ok(query_as!(
            CustomReaction,
            "SELECT * FROM reaction WHERE trigger = $1",
            trigger
        )
        .fetch_all(&self.get())
        .await?)
    }

    pub async fn fetch_notes_for(&self, userid: u64) -> Result<Vec<Note>> {
        Ok(query_as!(
            Note,
            "SELECT * FROM user_note WHERE user_id = $1",
            userid as i64
        )
        .fetch_all(&self.get())
        .await?)
    }

    pub async fn fetch_screenshares_for(&self, id: u64) -> Result<Option<Screenshare>> {
        Ok(query_as!(
            Screenshare,
            "SELECT * FROM screenshare WHERE in_question = $1 OR creator_id = $1",
            id as i64
        )
        .fetch_optional(&self.get())
        .await?)
    }

    pub async fn fetch_freezes_for(&self, id: u64) -> Result<Option<Freeze>> {
        Ok(query_as!(
            Freeze,
            "SELECT * FROM freezes WHERE user_id = $1",
            id as i64
        )
        .fetch_optional(&self.get())
        .await?)
    }

    pub async fn add_custom_reaction(&self, id: u64, emoji: &str, trigger: &str) -> Result<()> {
        query!(
            "INSERT INTO reaction (user_id, emoji, trigger) VALUES ($1, $2, $3)",
            id as i64,
            emoji,
            trigger
        )
        .execute(&self.get())
        .await?;
        Ok(())
    }

    pub async fn add_scrim_unban(
        &self,
        id: u64,
        unban_date: Option<OffsetDateTime>,
        roles: &[u64],
    ) -> Result<()> {
        query!(
            "INSERT INTO scheduled_scrim_unban (user_id, expires_at, roles) VALUES ($1, $2, $3)",
            id as i64,
            unban_date,
            &roles.iter().map(|r| *r as i64).collect::<Vec<_>>()
        )
        .execute(&self.get())
        .await?;
        Ok(())
    }

    pub async fn modify_scrim_unban(
        &self,
        id: u64,
        unban_date: Option<OffsetDateTime>,
        roles: &[u64],
    ) -> Result<()> {
        query!(
            "UPDATE scheduled_scrim_unban SET expires_at = $2, roles = $3 WHERE user_id = $1",
            id as i64,
            unban_date,
            &roles.iter().map(|r| *r as i64).collect::<Vec<_>>(),
        )
        .execute(&self.get())
        .await?;
        Ok(())
    }

    pub async fn add_note(
        &self,
        userid: u64,
        created_at: OffsetDateTime,
        note: &str,
        creator: u64,
    ) -> Result<usize> {
        let mut count = query!("SELECT * FROM user_note WHERE user_id = $1", userid as i64)
            .fetch_all(&self.get())
            .await?
            .len();
        query!("INSERT INTO user_note (user_id, id, created_at, note, creator) VALUES ($1, $2, $3, $4, $5)", userid as i64, count as i32 + 1, created_at, note, creator as i64).execute(&self.get()).await?;
        Ok(count)
    }

    pub async fn add_screenshare(&self, id: u64, creator: u64, in_question: u64) -> Result<()> {
        query!(
            "INSERT INTO screenshare (channel_id, creator_id, in_question) VALUES ($1, $2, $3)",
            id as i64,
            creator as i64,
            in_question as i64,
        )
        .execute(&self.get())
        .await?;
        Ok(())
    }

    pub async fn add_freeze(&self, id: u64, roles: &[u64]) -> Result<()> {
        query!(
            "INSERT INTO freezes (user_id, roles) VALUES ($1, $2)",
            id as i64,
            &roles.iter().map(|r| *r as i64).collect::<Vec<_>>(),
        )
        .execute(&self.get())
        .await?;
        Ok(())
    }

    pub async fn remove_note(&self, userid: u64, id: u64) -> Result<()> {
        query!(
            "DELETE FROM user_note WHERE user_id = $1 and id = $2",
            userid as i64,
            id as i32
        )
        .execute(&self.get())
        .await?;
        query!(
            "UPDATE user_note SET id = id - 1 WHERE user_id = $1 and id >= $2",
            userid as i64,
            id as i32
        )
        .execute(&self.get())
        .await?;
        Ok(())
    }

    pub async fn remove_custom_reaction(&self, user: u64) -> Result<()> {
        query!("DELETE FROM reaction WHERE user_id = $1", user as i64)
            .execute(&self.get())
            .await?;
        Ok(())
    }

    pub async fn get_screensharers(&self) -> Result<Vec<Screensharer>> {
        Ok(query_as!(Screensharer, "SELECT * FROM screensharer_stats")
            .fetch_all(&self.get())
            .await?)
    }

    pub async fn get_screensharer(&self, user: u64) -> Result<Option<Screensharer>> {
        Ok(query_as!(
            Screensharer,
            "SELECT * FROM screensharer_stats WHERE user_id = $1",
            user as i64,
        )
        .fetch_optional(&self.get())
        .await?)
    }

    pub async fn set_screensharer(&self, sc: Screensharer) -> Result<()> {
        if self.get_screensharer(sc.user_id).await?.is_some() {
            query!(
                "UPDATE screensharer_stats SET freezes = $1 WHERE user_id = $2",
                sc.freezes,
                sc.user_id as i64,
            )
            .execute(&self.get())
            .await?;
        } else {
            query!(
                "INSERT INTO screensharer_stats (user_id, freezes) VALUES ($1, $2)",
                sc.user_id as i64,
                sc.freezes,
            )
            .execute(&self.get())
            .await?;
        }
        Ok(())
    }
}
