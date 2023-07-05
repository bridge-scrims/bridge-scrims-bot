use std::fmt::Display;
use std::num::ParseIntError;

use serenity::model::id::{ChannelId, GuildId, RoleId, UserId};
use time::OffsetDateTime;

pub struct ScrimUnban {
    pub id: u64,
    pub date: Option<OffsetDateTime>,
    pub roles: Ids,
}

impl ScrimUnban {
    pub fn is_expired(&self) -> bool {
        self.date
            .map_or(true, |date| date <= OffsetDateTime::now_utc())
    }

    pub fn was_logged(&self) -> bool {
        self.date.is_none()
    }
}

pub struct Screenshare {
    /// Channel ID of the ticket
    pub id: u64,
    /// User ID of the person who made the ticket
    pub creator: u64,
    /// User ID of the person being screenshared
    pub in_question: u64,
}

pub struct Freeze {
    /// User ID of the person being frozen
    pub id: u64,
    /// Their roles
    pub roles: Vec<RoleId>,
    /// Time when they were frozen
    pub time: OffsetDateTime,
}

#[derive(Debug)]
pub struct Note {
    /// the id of the person that the note belongs to
    pub userid: u64,
    /// the note id
    pub id: u64,
    /// the date that the note was created at
    pub created_at: OffsetDateTime,
    /// the text that the note contains
    pub note: String,
    /// the id of the person who added the note
    pub creator: u64,
}

pub struct Ids(pub Vec<u64>);

impl Display for Ids {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Avoid allocating a string for each role id
        for i in 0..self.0.len() {
            let sep = if i == self.0.len() - 1 { "" } else { "," };

            write!(f, "{}{}", self.0[i], sep)?;
        }
        Ok(())
    }
}

impl TryFrom<String> for Ids {
    type Error = ParseIntError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(Self(Vec::new()));
        }
        let mut ids = Vec::new();
        for id in value.split(',') {
            let id = id.parse::<u64>()?;
            ids.push(id);
        }

        Ok(Self(ids))
    }
}

crate::id_impl!(Ids, UserId, GuildId, ChannelId, RoleId);

pub struct CustomReaction {
    pub user: u64,
    pub trigger: String,
    pub emoji: String,
}

pub struct Screensharer {
    pub id: u64,
    pub freezes: u64,
}
