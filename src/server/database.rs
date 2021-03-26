
use std::{sync::{Arc, Mutex}, time::UNIX_EPOCH};

use rusqlite::{Connection, NO_PARAMS, params};

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

        ResultsDb {
            connection,
        }
    }

    pub(super) fn add_entry(&self, res: OnionResultAggregated) {
        let connection = self.connection.lock().unwrap();
        add_entry(&connection, res);
    }

}


pub fn create_or_open() -> Connection {
    let db = Connection::open(DB_NAME).expect("Could not open the DB");

    db.execute(
        "CREATE TABLE IF NOT EXISTS onion_results(
        timestamp TEXT,
        total INTEGER NOT NULL,
        successful INTEGER NOT NULL
    )",
        NO_PARAMS,
    )
    .expect("could not create or open DB");

    db
}


pub(super) fn add_entry(db: &Connection, res: OnionResultAggregated) {

    let ms_from_epoch = res.time
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