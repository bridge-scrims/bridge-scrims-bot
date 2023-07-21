use time::OffsetDateTime;

pub struct ScrimUnban {
    pub user_id: u64,
    pub expires_at: Option<OffsetDateTime>,
    pub roles: Vec<u64>,
}

impl ScrimUnban {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map_or(true, |date| date <= OffsetDateTime::now_utc())
    }

    pub fn was_logged(&self) -> bool {
        self.expires_at.is_none()
    }
}

pub struct Screenshare {
    /// Channel ID of the ticket
    pub channel_id: u64,
    /// User ID of the person who made the ticket
    pub creator_id: u64,
    /// User ID of the person being screenshared
    pub in_question: u64,
}

pub struct Freeze {
    /// User ID of the person being frozen
    pub user_id: u64,
    /// Their roles
    pub roles: Vec<u64>,
}

#[derive(Debug)]
pub struct Note {
    /// the id of the person that the note belongs to
    pub user_id: u64,
    /// the note id
    pub id: u64,
    /// the date that the note was created at
    pub created_at: OffsetDateTime,
    /// the text that the note contains
    pub note: String,
    /// the id of the person who added the note
    pub creator: u64,
}

pub struct CustomReaction {
    pub user_id: u64,
    pub trigger: String,
    pub emoji: String,
}

pub struct Screensharer {
    pub user_id: u64,
    pub freezes: i32,
}
