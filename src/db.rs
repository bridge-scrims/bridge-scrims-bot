use std::sync::Mutex;

use sqlite::Connection;
use time::OffsetDateTime;

pub type Unban = (u64, OffsetDateTime);

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

        let conn = Connection::open(path.to_path_buf()).unwrap();

        // Tables
        conn.execute(
            "create table if not exists ScheduledUnbans (
                id integer primary key,
                time integer
            )",
        )
        .expect("Could not initialize database");

        Self {
            sqlite: Mutex::new(conn),
        }
    }

    pub async fn fetch_unbans(&self) -> Vec<Unban> {
        let mut result = Vec::new();
        let _lock = self.sqlite.lock().map(|db| {
            let stmt = db.prepare("SELECT * FROM 'ScheduledUnbans'");
            if let Ok(stmt) = stmt {
                let mut cursor = stmt.into_cursor();

                while let Ok(Some(row)) = cursor.next() {
                    tracing::debug!("Got row: {:?}", &row);
                    let user_id = row.get(0).unwrap().as_integer().unwrap() as u64;
                    let time = row.get(1).unwrap().as_integer().unwrap();
                    let offset_date_time = OffsetDateTime::from_unix_timestamp(time).unwrap();
                    result.push((user_id, offset_date_time));
                }
            }
        });
        result
    }

    pub fn add_unban(&self, id: u64, unban_date: OffsetDateTime) -> Result<(), sqlite::Error> {
        let result = self
            .sqlite
            .lock()
            .map(|db| {
                db.execute(format!(
                    "INSERT INTO ScheduledUnbans (id,time) values ({},{})",
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

    pub fn remove_unban(&self, i: u64) -> Result<(), sqlite::Error> {
        let result = self
            .sqlite
            .lock()
            .map(|db| db.execute(format!("DELETE FROM 'ScheduledUnbans' WHERE id = {}", i)))
            .ok();

        if let Some(result) = result {
            result
        } else {
            Ok(())
        }
    }
}
