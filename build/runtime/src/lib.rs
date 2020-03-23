use bibifi_database::Database;
use bibifi_parser::parse;
use bibifi_parser::types::Program;
use std::error::Error;
use std::sync::mpsc::Sender;

pub struct BiBiFi {
    database: Database,
}

impl BiBiFi {
    pub fn new(admin_hash: [u8; 32]) -> BiBiFi {
        BiBiFi {
            database: Database::new(admin_hash),
        }
    }

    pub fn run(program: String, stream: Sender<String>) {
        let program = parse(program);
        match program {
            Ok(program) => {
                // TODO handle program
            }
            Err(_) => {}
        }
    }
}
