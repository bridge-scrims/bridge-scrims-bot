use std::{
    cmp::Reverse,
    str::FromStr,
    sync::{Mutex, MutexGuard},
};

use serenity::model::id::RoleId;
use sqlx::{postgres::PgPoolOptions, PgPool};
use time::OffsetDateTime;

pub use crate::model::*;

pub struct Database(pub PgPool);

impl Database {
    pub async fn init() -> crate::Result<Database> {
        Ok(Self(
            PgPoolOptions::new()
                .connect(&std::env::var("DATABASE_URL")?)
                .await?,
        ))
    }

    pub fn fetch_scrim_unbans(&self) -> Vec<ScrimUnban> {
        let mut result = Vec::new();
        self.fetch_rows("ScheduledScrimUnbans", "", |row| {
            let id = row.get(0).unwrap().as_integer().unwrap() as u64;
            let time = row.get(1).unwrap().as_integer();
            let date = time.map(|v| OffsetDateTime::from_unix_timestamp(v).unwrap());
            let roles = Ids::try_from(row.get(2).unwrap().as_string().unwrap().to_owned()).unwrap();

            result.push(ScrimUnban { id, date, roles });
        });
        result
    }

    pub fn fetch_custom_reactions(&self) -> Vec<CustomReaction> {
        let mut result = Vec::new();
        self.fetch_rows("Reaction", "", |row| {
            let user = row.get(0).unwrap().as_integer().unwrap() as u64;
            let emoji = row.get(1).unwrap().as_string().unwrap().to_string();
            let trigger = row.get(2).unwrap().as_string().unwrap().to_string();

            result.push(CustomReaction {
                user,
                emoji,
                trigger,
            });
        });
        result
    }

    pub fn fetch_custom_reactions_for(&self, userid: u64) -> Vec<CustomReaction> {
        let mut result = Vec::new();
        self.fetch_rows("Reaction", &format!("where user = {}", userid), |row| {
            let user = row.get(0).unwrap().as_integer().unwrap() as u64;
            let emoji = row.get(1).unwrap().as_string().unwrap().to_string();
            let trigger = row.get(2).unwrap().as_string().unwrap().to_string();

            result.push(CustomReaction {
                user,
                emoji,
                trigger,
            });
        });
        result
    }

    pub fn fetch_custom_reactions_with_trigger(
        &self,
        trigger: &str,
    ) -> SqliteResult<Vec<CustomReaction>> {
        self.fetch_rows_safe(
            "SELECT * FROM 'Reaction' WHERE trigger = ?",
            |stmt| stmt.bind(1, trigger),
            |row| {
                let user = row.get(0).unwrap().as_integer().unwrap() as u64;
                let emoji = row.get(1).unwrap().as_string().unwrap().to_string();
                let trigger = row.get(2).unwrap().as_string().unwrap().to_string();

                CustomReaction {
                    user,
                    emoji,
                    trigger,
                }
            },
        )
    }

    pub fn fetch_notes_for(&self, userid: u64) -> Vec<Note> {
        let mut result = Vec::new();
        self.fetch_rows("Notes", &format!("where userid = {}", userid), |row| {
            let userid = row.get(0).unwrap().as_integer().unwrap() as u64;
            let id = row.get(1).unwrap().as_integer().unwrap() as u64;
            let time = row.get(2).unwrap().as_integer().unwrap();
            let created_at = OffsetDateTime::from_unix_timestamp(time).unwrap();
            let note = row.get(3).unwrap().as_string().unwrap().to_string();
            let creator = row.get(4).unwrap().as_integer().unwrap() as u64;

            result.push(Note {
                userid,
                id,
                created_at,
                note,
                creator,
            });
        });
        result
    }

    pub fn fetch_screenshares_for(&self, id: u64) -> Option<Screenshare> {
        let mut result = None;
        self.fetch_rows(
            "Screenshares",
            &format!("where id = {id} OR creator = {id}"),
            |row| {
                let creator = row[1].as_integer().unwrap() as u64;
                let in_question = row[2].as_integer().unwrap() as u64;
                result.get_or_insert(Screenshare {
                    id,
                    creator,
                    in_question,
                });
            },
        );
        result
    }

    pub fn fetch_freezes_for(&self, id: u64) -> Option<Freeze> {
        let mut result = None;
        self.fetch_rows("Freezes", &format!("where id = {}", id), |row| {
            let roles = row[1]
                .as_string()
                .unwrap_or_default()
                .split(',')
                .filter_map(|x| RoleId::from_str(x).ok())
                .collect();
            let time = OffsetDateTime::from_unix_timestamp(row[2].as_integer().unwrap()).unwrap();
            result.get_or_insert(Freeze { id, roles, time });
        });
        result
    }

    pub fn add_custom_reaction(&self, id: u64, emoji: &str, trigger: &str) -> SqliteResult {
        self.exec_safe(
            "INSERT INTO 'Reaction' (user, emoji, trigger) values (?, ?, ?)",
            |stmt| {
                stmt.bind(1, id as i64)?;
                stmt.bind(2, emoji)?;
                stmt.bind(3, trigger)
            },
        )
    }

    pub fn add_scrim_unban(
        &self,
        id: u64,
        unban_date: Option<OffsetDateTime>,
        roles: &Ids,
    ) -> SqliteResult {
        self.get_lock(|db| {
            db.execute(format!(
                "INSERT INTO 'ScheduledScrimUnbans' (id, time, roles) values ({}, {}, \"{}\")",
                id,
                unban_date.map_or(String::from("NULL"), |date| format!(
                    "\"{}\"",
                    date.unix_timestamp()
                )),
                roles,
            ))
        })
    }

    pub fn modify_scrim_unban(
        &self,
        id: u64,
        unban_date: Option<OffsetDateTime>,
        roles: &Ids,
    ) -> SqliteResult {
        self.get_lock(|db| {
            db.execute(format!(
                "UPDATE 'ScheduledScrimUnbans' SET time = {}, roles = \"{}\" WHERE id = {}",
                unban_date.map_or(String::from("NULL"), |date| format!(
                    "\"{}\"",
                    date.unix_timestamp()
                )),
                roles,
                id,
            ))
        })
    }

    pub fn add_note(
        &self,
        userid: u64,
        created_at: OffsetDateTime,
        note: &str,
        creator: u64,
    ) -> SqliteResult<i64> {
        let mut count: Option<i64> = None;
        self.count_rows("Notes", &format!("where userid = {}", userid), |val| {
            if let sqlite::Value::Integer(co) = val[0] {
                count = Some(co);
            }
        });
        let count = count.unwrap_or_default();
        self.exec_safe(
            "INSERT INTO 'Notes' (userid, id, created_at, note, creator) values (?, ?, ?, ?, ?)",
            |stmt| {
                stmt.bind(1, userid as i64)?;
                stmt.bind(2, count + 1)?;
                stmt.bind(3, created_at.unix_timestamp())?;
                stmt.bind(4, note)?;
                stmt.bind(5, creator as i64)
            },
        )
        .map(|_| count + 1)
    }

    pub fn add_screenshare(&self, id: u64, creator: u64, in_question: u64) -> SqliteResult {
        self.get_lock(|db| {
            db.execute(format!(
                "INSERT INTO 'Screenshares' (id,creator,in_question) values ({},{},{})",
                id, creator, in_question
            ))
        })
    }

    pub fn add_freeze(&self, id: u64, roles: Ids, time: OffsetDateTime) -> SqliteResult {
        self.get_lock(|db| {
            db.execute(format!(
                "INSERT INTO 'Freezes' (id,roles,time) values ({},\"{}\",{})",
                id,
                roles,
                time.unix_timestamp(),
            ))
        })
    }

    pub fn remove_note(&self, userid: u64, id: u64) -> SqliteResult {
        self.get_lock(|db| {
            db.execute(format!(
                "DELETE FROM 'Notes' WHERE userid = {} AND id = {}",
                userid, id
            ))?;
            db.execute(format!(
                "UPDATE 'Notes' SET id = id - 1 WHERE userid = {} AND id >= {}",
                userid, id
            ))
        })
    }

    pub fn remove_entry(&self, table: &str, i: u64) -> SqliteResult {
        self.get_lock(|db| db.execute(format!("DELETE FROM '{}' WHERE id = {}", table, i)))
    }

    pub fn remove_custom_reaction(&self, user: u64) -> Result<(), sqlite::Error> {
        let result = self
            .sqlite
            .lock()
            .map(|db| db.execute(format!("DELETE FROM 'Reaction' WHERE user = {}", user)))
            .ok();
        if let Some(result) = result {
            result
        } else {
            Ok(())
        }
    }

    pub fn get_screensharers(&self) -> Vec<Screensharer> {
        let mut result = Vec::new();
        self.fetch_rows("ScreensharerStats", "", |screensharer| {
            let id = screensharer.get(0).unwrap().as_integer().unwrap() as u64;
            let freezes = screensharer.get(1).unwrap().as_integer().unwrap() as u64;
            result.push(Screensharer { id, freezes });
        });
        result.sort_unstable_by_key(|x| Reverse(x.freezes));
        result
    }

    pub fn get_screensharer(&self, user: u64) -> Option<Screensharer> {
        let mut result = None;
        self.fetch_rows(
            "ScreensharerStats",
            &format!("where id = {}", user),
            |screensharer| {
                let id = screensharer.get(0).unwrap().as_integer().unwrap() as u64;
                let freezes = screensharer.get(1).unwrap().as_integer().unwrap() as u64;
                let _ = result.get_or_insert(Screensharer { id, freezes });
            },
        );
        result
    }

    pub fn set_screensharer(&self, sc: Screensharer) -> SqliteResult<()> {
        if self.get_screensharer(sc.id).is_some() {
            self.get_lock(|db| {
                db.execute(format!(
                    "UPDATE 'ScreensharerStats' SET freezes = {} WHERE id = {}",
                    sc.freezes, sc.id
                ))
            })
        } else {
            self.get_lock(|db| {
                db.execute(format!(
                    "INSERT INTO 'ScreensharerStats' (id,freezes) VALUES ({}, {})",
                    sc.id, sc.freezes
                ))
            })
        }
    }
}
