use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, NO_PARAMS};

use log::error;

use super::OnionResultAggregated;

const DB_NAME: &'static str = "data.db";

#[derive(Debug)]
pub struct ResultsDb {
    connection: Arc<Mutex<Connection>>,
}

impl ResultsDb {
    pub fn new() -> Self {
        let connection = create_or_open();

        let connection = Arc::new(Mutex::new(connection));

        ResultsDb { connection }
    }

    pub(super) fn add_entry(&self, res: OnionResultAggregated) {
        let connection = self.connection.lock().unwrap();
        add_entry(&connection, res);
    }

    pub(super) fn read_results(&self) -> Vec<OnionResultAggregated> {
        let connection = self.connection.lock().unwrap();
        get_entries(&connection)
    }
}

pub fn create_or_open() -> Connection {
    let db = Connection::open(DB_NAME).expect("Could not open the DB");

    db.execute(
        "CREATE TABLE IF NOT EXISTS onion_results(
        timestamp TEXT NOT NULL PRIMARY KEY,
        total INTEGER NOT NULL,
        successful INTEGER NOT NULL
    )",
        NO_PARAMS,
    )
    .expect("could not create or open DB");

    db
}

pub(super) fn add_entry(db: &Connection, res: OnionResultAggregated) {
    let ms_from_epoch = res
        .time
        .duration_since(UNIX_EPOCH)
        .expect("Could not get UNIX time")
        .as_millis();

    let timestamp = ms_from_epoch.to_string();

    if let Err(error) = db.execute(
        "INSERT INTO onion_results (timestamp, total, successful) values (?1, ?2, ?3)",
        params![timestamp, res.total, res.total_success],
    ) {
        eprintln!("Could not insert: {}", error);
    }
}

fn get_entries(db: &Connection) -> Vec<OnionResultAggregated> {
    let mut stmt = db
        .prepare(
            "SELECT timestamp, total, successful FROM onion_results ORDER BY timestamp LIMIT 720",
        )
        .expect("Failed to prepare db statement");

    let results: Vec<_> = stmt
        .query_map(params![], |row| {
            let ms_str: String = row.get(0)?;
            let ms = ms_str.parse::<u64>().unwrap();

            let time = SystemTime::UNIX_EPOCH
                .checked_add(Duration::from_millis(ms))
                .unwrap();

            Ok(OnionResultAggregated {
                time,
                total: row.get(1)?,
                total_success: row.get(2)?,
            })
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect();

    results
}
