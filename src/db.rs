pub use crate::model::*;
use std::{
    str::FromStr,
    sync::{Mutex, MutexGuard}, cmp::Reverse,
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

        conn.execute(
            "create table if not exists Screenshares (
                id integer primary key,
                creator integer,
                in_question integer
            )",
        )
        .expect("Could not initialize database");

        conn.execute(
            "create table if not exists Freezes (
                id integer,
                roles text,
                time integer
            )",
        )
        .expect("Could not initialize database");

        conn.execute(
            "create table if not exists ScreensharerStats (
                id integer primary key,
                freezes integer
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

    pub fn fetch_custom_reactions_with_trigger(&self, trigger: &str) -> Vec<CustomReaction> {
        let mut result = Vec::new();
        self.fetch_rows("Reaction", &format!("where trigger = '{}'", trigger), |row| {
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

    pub fn fetch_screenshares_for(&self, id: u64) -> Option<Screenshare> {
        let mut result = None;
        self.fetch_rows("Screenshares", &format!("where id = {}", id), |row| {
            let creator = row[1].as_integer().unwrap() as u64;
            let in_question = row[2].as_integer().unwrap() as u64;
            result.get_or_insert(Screenshare {
                id,
                creator,
                in_question,
            });
        });
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

    pub fn add_unban(&self, id: u64, unban_date: OffsetDateTime) -> SqliteResult {
        self.get_lock(|db| {
            db.execute(format!(
                "INSERT INTO 'ScheduledUnbans' (id,time) values ({},{})",
                id,
                unban_date.unix_timestamp()
            ))
        })
    }

    pub fn modify_unban_date(
        &self,
        table: &str,
        id: u64,
        unban_date: OffsetDateTime,
    ) -> SqliteResult {
        self.get_lock(|db| {
            db.execute(format!(
                "UPDATE '{}' SET time = {} WHERE id = {}",
                table,
                unban_date.unix_timestamp(),
                id,
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
        roles: &Ids,
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
        self.fetch_rows(
            "ScreensharerStats",
            "",
            |screensharer| {
                let id = screensharer.get(0).unwrap().as_integer().unwrap() as u64;
                let freezes = screensharer.get(1).unwrap().as_integer().unwrap() as u64;
                result.push(Screensharer { id, freezes });
            },
        );
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
                let _ = result.get_or_insert_with(|| Screensharer { id, freezes });
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
                    sc.freezes, sc.id
                ))
            })
        }
    }
}
