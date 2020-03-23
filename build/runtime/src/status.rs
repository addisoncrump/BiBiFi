use bibifi_database::Value;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Entry {
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Debug, Eq, PartialEq)]
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
