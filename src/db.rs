use std::{
    fmt::Display,
    num::ParseIntError,
    sync::{Mutex, MutexGuard},
};

use serenity::model::id::RoleId;
use sqlite::Connection;
use time::OffsetDateTime;

type SqliteResult<T = ()> = Result<T, sqlite::Error>;
pub struct Database {
    pub sqlite: Mutex<Connection>,
}

impl Database {
    pub fn init() -> Self {
        // Create sqlite file
        let mut path = crate::consts::DATABASE_PATH.clone();
        let _ = std::fs::create_dir_all(&path);
        path.push("bridge-scrims.sqlite");
        if !path.as_path().exists() {
            std::fs::File::create(&path).expect("Cannot create database file");
        }

        let conn = Connection::open(&path).unwrap();

        // Tables
        conn.execute(
            "create table if not exists ScheduledUnbans (
                id integer primary key,
                time integer
            )",
        )
        .expect("Could not initialize database");

        conn.execute(
            "create table if not exists ScheduledScrimUnbans (
                id integer primary key,
                time integer,
                roles text
            )",
        )
        .expect("Could not initialize database");

        conn.execute(
            "create table if not exists Notes (
                userid integer,
                id integer,
                created_at integer,
                note text,
                creator integer
            )",
        )
        .expect("Could not initialize database");

        conn.execute(
            "create table if not exists Reaction (
                user integer,
                emoji text,
                trigger text
            )",
        )
        .expect("Could not initialize database");

        Self {
            sqlite: Mutex::new(conn),
        }
    }

    pub fn get_lock<T, F>(&self, predicate: F) -> SqliteResult<T>
    where
        F: FnOnce(MutexGuard<Connection>) -> SqliteResult<T>,
    {
        self.sqlite.lock().map_or_else(
            |_| {
                Err(sqlite::Error {
                    code: Some(6),
                    message: Some("Could not lock database".to_string()),
                })
            },
            predicate,
        )
    }

    pub fn fetch_rows<F>(&self, table: &str, condition: &str, mut predicate: F)
    where
        F: FnMut(&[sqlite::Value]),
    {
        let _lock = self.get_lock(|db| {
            let stmt = db.prepare(format!("SELECT * FROM '{}' {}", table, condition));
            if let Ok(stmt) = stmt {
                let mut cursor = stmt.into_cursor();

                while let Ok(Some(row)) = cursor.next() {
                    predicate(row);
                }
            }
            Ok(())
        });
    }

    pub fn count_rows<F>(&self, table: &str, condition: &str, mut predicate: F)
    where
        F: FnMut(&[sqlite::Value]),
    {
        let _lock = self.get_lock(|db| {
            let stmt = db.prepare(format!("select count(*) from '{}' {}", table, condition));
            if let Ok(stmt) = stmt {
                let mut cursor = stmt.into_cursor();

                if let Ok(Some(row)) = cursor.next() {
                    predicate(row);
                }
            }
            Ok(())
        });
    }

    pub fn fetch_unbans(&self) -> Vec<Unban> {
        tracing::info!("Fetching bans");
        let mut result = Vec::new();
        self.fetch_rows("ScheduledUnbans", "", |row| {
            let id = row.get(0).unwrap().as_integer().unwrap() as u64;
            let time = row.get(1).unwrap().as_integer().unwrap();
            let date = OffsetDateTime::from_unix_timestamp(time).unwrap();
            result.push(Unban { id, date });
        });
        result
    }

    pub fn fetch_scrim_unbans(&self) -> Vec<ScrimUnban> {
        tracing::info!("Fetching scrim bans");
        let mut result = Vec::new();
        self.fetch_rows("ScheduledScrimUnbans", "", |row| {
            let id = row.get(0).unwrap().as_integer().unwrap() as u64;
            let time = row.get(1).unwrap().as_integer().unwrap();
            let date = OffsetDateTime::from_unix_timestamp(time).unwrap();
            let roles =
                BanRoles::try_from(row.get(2).unwrap().as_string().unwrap().to_owned()).unwrap();

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

    pub fn add_unban(&self, id: u64, unban_date: OffsetDateTime) -> SqliteResult {
        self.get_lock(|db| {
            db.execute(format!(
                "INSERT INTO 'ScheduledUnbans' (id,time) values ({},{})",
                id,
                unban_date.unix_timestamp()
            ))
        })
    }
    pub fn add_custom_reaction(
        &self,
        id: u64,
        emoji: &str,
        trigger: &str,
    ) -> Result<(), sqlite::Error> {
        let result = self
            .sqlite
            .lock()
            .map(|db| {
                db.execute(format!(
                    "INSERT INTO 'Reaction' (user,emoji,trigger) values ({}, \"{}\", \"{}\")",
                    id, emoji, trigger
                ))
            })
            .ok();
        if let Some(result) = result {
            result
        } else {
            Ok(())
        }
    }

    pub fn add_scrim_unban(
        &self,
        id: u64,
        unban_date: OffsetDateTime,
        roles: &BanRoles,
    ) -> SqliteResult {
        self.get_lock(|db| {
            db.execute(format!(
                "INSERT INTO 'ScheduledScrimUnbans' (id,time,roles) values ({},{},\"{}\")",
                id,
                unban_date.unix_timestamp(),
                roles,
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
        self.get_lock(|db| {
            db.execute(format!(
                "INSERT INTO 'Notes' (userid,id,created_at,note,creator) values ({},{},\"{}\",\"{}\",{})",
                userid,
                count+1,
                created_at.unix_timestamp(),
                note,
                creator
            )).map(|_| count + 1)
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
}

pub struct Unban {
    pub id: u64,
    pub date: OffsetDateTime,
}

pub struct ScrimUnban {
    pub id: u64,
    pub date: OffsetDateTime,
    pub roles: BanRoles,
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

pub struct CustomReaction {
    pub user: u64,
    pub trigger: String,
    pub emoji: String,
}

pub struct BanRoles(pub Vec<RoleId>);

impl Display for BanRoles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Avoid allocating a string for each role id
        for i in 0..self.0.len() {
            let sep = if i == self.0.len() - 1 { "" } else { "," };

            write!(f, "{}{}", self.0[i], sep)?;
        }
        Ok(())
    }
}

impl TryFrom<String> for BanRoles {
    type Error = ParseIntError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(Self(Vec::new()));
        }
        let mut roles = Vec::new();
        for role in value.split(',') {
            let role = role.parse::<u64>()?;
            roles.push(RoleId(role));
        }

        Ok(Self(roles))
    }
}
