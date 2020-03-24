use super::*;
use crate::status::Status::*;
use bibifi_database::Database;
use bibifi_util::hash;
use tokio::sync::mpsc::unbounded_channel;

#[tokio::test]
async fn example() {
    let db = Database::new(hash("admin".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal admin password "admin" do
    exit
    ***"#;
    match BiBiFi::run_program(db.clone(), program.to_string(), sender).await {
        None => assert!(false),
        Some(db1) => {
            assert_eq!(db, db1);
            assert_eq!(
                Entry {
                    status: EXITING,
                    output: None
                },
                receiver.recv().await.unwrap()
            );
        }
    }
}

// parsing errors take precedence over security violation
// test with invalid syntax and incorrect password should return FAILED
#[tokio::test]
async fn t1_parse_err_vs_sec_fail() {
    let db_in = Database::new(hash("admin_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal admin password "admin" do exit
    ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => {
            assert_eq!(
                Entry { status: FAILED, output: None },
                receiver.recv().await.unwrap()
            );
        },
        Some(_) => assert!(false),
    }
}


// acting principal has to be there in database
// test with non-existing acting principal should return FAILED
#[tokio::test]
async fn t2_non_existing_acting_principal() {
    let db_in = Database::new(hash("admin_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal bob password "admin_pass" do
                            exit
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => {
            assert_eq!(
                Entry { status: FAILED, output: None },
                receiver.recv().await.unwrap()
            );
        },
        Some(_) => assert!(false)
    }
}


// acting principal cannot be anyone
// test with anyone acting principal should return FAILED
#[tokio::test]
async fn t3_anyone_acting_principal() {
    let db_in = Database::new(hash("admin_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal anyone password "admin_pass" do
                            exit
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => {
            assert_eq!(
                Entry { status: FAILED, output: None },
                receiver.recv().await.unwrap()
            );
        },
        Some(_) => assert!(false)
    }
}


// acting principal should have correct password
// test with incorrect password should return DENIED
#[tokio::test]
async fn t4_acting_p_inc_pass() {
    let db_in = Database::new(hash("admin_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal admin password "admin" do
                            exit
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => {
            assert_eq!(
                receiver.recv().await.unwrap(),
                Entry { status: DENIED, output: None }
            );
        },
        Some(_) => assert!(false)
    }
}

