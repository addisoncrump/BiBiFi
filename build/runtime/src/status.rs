use bibifi_database::{Status as DBStatus, Value};
use serde::Serialize;

#[derive(Clone, Serialize, Debug, Eq, PartialEq)]
pub struct Entry {
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Serialize, Debug, Eq, PartialEq)]
pub enum Status {
    CREATE_PRINCIPAL,
    CHANGE_PASSWORD,
    SET,
    APPEND,
    LOCAL,
    FOREACH,
    SET_DELEGATION,
    DELETE_DELEGATION,
    DEFAULT_DELEGATOR,
    DENIED,
    FAILED,
    RETURNING,
    EXITING,
}

impl Entry {
    pub fn from(status: DBStatus, success_status: Status) -> Self {
        match status {
            DBStatus::SUCCESS => Entry {
                status: success_status,
                output: None,
            },
            DBStatus::DENIED => Entry {
                status: Status::DENIED,
                output: None,
            },
            DBStatus::FAILED => Entry {
                status: Status::FAILED,
                output: None,
            },
        }
    }
}
