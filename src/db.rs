use std::{fmt::Display, num::ParseIntError, sync::Mutex};

use serenity::model::id::RoleId;
use sqlite::Connection;
use time::OffsetDateTime;

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

        Self {
            sqlite: Mutex::new(conn),
        }
    }

    pub fn fetch_rows<F>(&self, table: &str, condition: &str, mut predicate: F)
    where
        F: FnMut(&[sqlite::Value]),
    {
        let _lock = self.sqlite.lock().map(|db| {
            let stmt = db.prepare(format!("SELECT * FROM '{}' {}", table, condition));
            if let Ok(stmt) = stmt {
                let mut cursor = stmt.into_cursor();

                while let Ok(Some(row)) = cursor.next() {
                    predicate(row);
                }
            }
        });
    }

    pub fn count_rows<F>(&self, table: &str, condition: &str, mut predicate: F)
    where
        F: FnMut(&[sqlite::Value]),
    {
        let _lock = self.sqlite.lock().map(|db| {
            let stmt = db.prepare(format!("select count(*) from '{}' {}", table, condition));
            if let Ok(stmt) = stmt {
                let mut cursor = stmt.into_cursor();

                if let Ok(Some(row)) = cursor.next() {
                    predicate(row);
                }
            }
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

    pub fn add_unban(&self, id: u64, unban_date: OffsetDateTime) -> Result<(), sqlite::Error> {
        let result = self
            .sqlite
            .lock()
            .map(|db| {
                db.execute(format!(
                    "INSERT INTO 'ScheduledUnbans' (id,time) values ({},{})",
                    id,
                    unban_date.unix_timestamp()
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
    ) -> Result<(), sqlite::Error> {
        let result = self
            .sqlite
            .lock()
            .map(|db| {
                db.execute(format!(
                    "INSERT INTO 'ScheduledScrimUnbans' (id,time,roles) values ({},{},\"{}\")",
                    id,
                    unban_date.unix_timestamp(),
                    roles,
                ))
            })
            .ok();
        if let Some(result) = result {
            result
        } else {
            Ok(())
        }
    }

    pub fn add_note(
        &self,
        userid: u64,
        created_at: OffsetDateTime,
        note: &str,
        creator: u64,
    ) -> Result<i64, std::sync::PoisonError<std::sync::MutexGuard<sqlite::Connection>>> {
        let mut count: Option<i64> = None;
        self.count_rows("Notes", &format!("where userid = {}", userid), |val| {
            if let sqlite::Value::Integer(co) = val[0] {
                count = Some(co);
            }
        });
        let count = count.unwrap_or_default();
        let result = self
            .sqlite
            .lock()
            .map(|db| {
                db.execute(format!(
                    "INSERT INTO 'Notes' (userid,id,created_at,note,creator) values ({},{},\"{}\",\"{}\",{})",
                    userid,
                    count+1,
                    created_at.unix_timestamp(),
                    note,
                    creator
                ))
            })
            .ok();
        if let Some(Ok(_)) = result {
            Ok(count + 1)
        } else {
            tracing::error!("{:?}", result);
            Ok(-1)
        }
    }

    pub fn remove_note(&self, userid: u64, id: u64) -> Result<(), sqlite::Error> {
        let result = self
            .sqlite
            .lock()
            .map(|db| {
                db.execute(format!(
                    "DELETE FROM 'Notes' WHERE userid = {} AND id = {}",
                    userid, id
                ))?;
                db.execute(format!(
                    "UPDATE 'Notes' SET id = id - 1 WHERE userid = {} AND id >= {}",
                    userid, id
                ))
            })
            .ok();
        if let Some(result) = result {
            result
        } else {
            Ok(())
        }
    }

    pub fn remove_entry(&self, table: &str, i: u64) -> Result<(), sqlite::Error> {
        let result = self
            .sqlite
            .lock()
            .map(|db| db.execute(format!("DELETE FROM '{}' WHERE id = {}", table, i)))
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