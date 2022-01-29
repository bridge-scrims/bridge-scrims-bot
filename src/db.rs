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

        Self {
            sqlite: Mutex::new(conn),
        }
    }

    pub fn fetch_rows<F>(&self, table: &str, mut predicate: F)
    where
        F: FnMut(&[sqlite::Value]),
    {
        let _lock = self.sqlite.lock().map(|db| {
            let stmt = db.prepare(format!("SELECT * FROM '{}'", table));
            if let Ok(stmt) = stmt {
                let mut cursor = stmt.into_cursor();

                while let Ok(Some(row)) = cursor.next() {
                    predicate(row);
                }
            }
        });
    }

    pub fn fetch_unbans(&self) -> Vec<Unban> {
        tracing::info!("Fetching bans");
        let mut result = Vec::new();
        self.fetch_rows("ScheduledUnbans", |row| {
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
        self.fetch_rows("ScheduledScrimUnbans", |row| {
            let id = row.get(0).unwrap().as_integer().unwrap() as u64;
            let time = row.get(1).unwrap().as_integer().unwrap();
            let date = OffsetDateTime::from_unix_timestamp(time).unwrap();
            let roles =
                BanRoles::try_from(row.get(2).unwrap().as_string().unwrap().to_owned()).unwrap();

            result.push(ScrimUnban { id, date, roles });
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
