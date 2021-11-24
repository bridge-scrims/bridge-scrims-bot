use std::sync::Mutex;

use sqlite::Connection;
use time::OffsetDateTime;

pub struct Database {
    pub sqlite: Mutex<Connection>,
    pub cache: DatabaseCache,
}

impl Database {
    pub fn init() -> Self {
        let mut path = crate::consts::DATABASE_PATH.clone();
        let _ = std::fs::create_dir_all(&path);
        path.push("bridge-scrims.sqlite");

        if !path.as_path().exists() {
            std::fs::File::create(&path).expect("Cannot create database file");
        }

        let conn = Connection::open(path.to_path_buf()).unwrap();

        conn.execute(
            "create table if not exists ScheduledUnbans (
                id integer primary key,
                time integer
            )",
        )
        .expect("Could not initialize database");

        Self {
            sqlite: Mutex::new(conn),
            cache: DatabaseCache::default(),
        }
    }

    pub async fn fetch_unbans(&mut self) {
        let _lock = self.sqlite.lock().map(|db| {
            self.cache.0.clear();
            let stmt = db.prepare("SELECT * FROM 'ScheduledUnbans'");
            if let Ok(stmt) = stmt {
                let mut cursor = stmt.into_cursor();

                while let Ok(Some(row)) = cursor.next() {
                    tracing::debug!("Got row: {:?}", &row);
                    let user_id = row.get(0).unwrap().as_integer().unwrap() as u64;
                    let time = row.get(1).unwrap().as_integer().unwrap();
                    let offset_date_time = OffsetDateTime::from_unix_timestamp(time).unwrap();
                    self.cache.0.push((user_id, offset_date_time));
                }
            }
        });
    }

    pub fn add_unban(&self, id: u64, unban_date: OffsetDateTime) {
        let _lock = self.sqlite.lock().map(|db| {
            let _ = db.execute(format!(
                "INSERT INTO ScheduledUnbans (id,time) values ({},{})",
                id,
                unban_date.unix_timestamp()
            ));
        });
    }

    pub fn remove_unban(&self, i: u64) {
        let _lock = self.sqlite.lock().map(|db| {
            let _ = db.execute(format!("DELETE FROM 'ScheduledUnbans' WHERE id = {}", i));
        });
    }
}

#[derive(Clone, Debug, Default)]
pub struct DatabaseCache(pub Vec<(u64, OffsetDateTime)>);
