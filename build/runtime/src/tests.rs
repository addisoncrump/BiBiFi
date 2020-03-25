use super::*;
use crate::status::Status::*;
use bibifi_database::Database;
use bibifi_util::hash;
use tokio::sync::mpsc::unbounded_channel;
use bibifi_database::Value;
use bibifi_database::Value::Immediate;

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
// test with anyone acting principal should return DENIED
#[tokio::test]
#[ignore]
async fn t3_anyone_acting_principal() {
    let db_in = Database::new(hash("admin_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal anyone password "admin_pass" do
                            exit
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => {
            assert_eq!(
                Entry { status: DENIED, output: None },
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

// only admin can give exit
// test with non-admin giving exit command should return DENIED
#[tokio::test]
async fn t5_non_admin_exit_cmd() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    db_in.create_principal(&"bob".to_string(), &hash("bob_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal bob password "bob_pass" do
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

// return command should return expr
// test with return cmd should return expr
#[tokio::test]
async fn t6_non_admin_exit_cmd() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    db_in.create_principal(&"bob".to_string(), &hash("bob_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal bob password "bob_pass" do
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => assert!(false),
        Some(db_out) => {
            assert_eq!(db_out, db_in);
            assert_eq!(
                receiver.recv().await.unwrap(),
                Entry { status: RETURNING, output: Some(Immediate("done".to_string())) }
            );
        },
    }
}


// create principal should create principal
#[tokio::test]
async fn t7_create_principal() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    let mut db_out_exp = db_in.clone();
    db_out_exp.create_principal(&"bob".to_string(), &hash("bob_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal admin password "admin_pass" do
                            create principal bob "bob_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => assert!(false),
        Some(db_out) => {
            assert_eq!(db_out, db_out_exp);
            assert_eq!(
                receiver.recv().await.unwrap(),
                Entry { status: CREATE_PRINCIPAL, output: None }
            );
            assert_eq!(
                receiver.recv().await.unwrap(),
                Entry { status: RETURNING, output: Some(Immediate("done".to_string())) }
            );
        },
    }
}

// only admin can create principal
// test with non-admin creating principal
#[tokio::test]
async fn t8_non_admin_create_principal() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    db_in.create_principal(&"bob".to_string(), &hash("bob_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal bob password "bob_pass" do
                            create principal alice "alice_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => {
            assert_eq!(
                receiver.recv().await.unwrap(),
                Entry { status: DENIED, output: None }
            );
        },
        Some(_) => assert!(false),
    }
}


// cannot create existing principal
// test with recreating principal
#[tokio::test]
async fn t9_recreate_principal() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    db_in.create_principal(&"bob".to_string(), &hash("bob_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal admin password "admin_pass" do
                            create principal anyone "anyone_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => {
            assert_eq!(
                receiver.recv().await.unwrap(),
                Entry { status: FAILED, output: None }
            );
        },
        Some(_) => assert!(false),
    }
}

// change password
#[tokio::test]
async fn t10_change_password() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    db_in.create_principal(&"bob".to_string(), &hash("bob_pass".to_string()));
    let mut db_out_exp = Database::new(hash("admin_pass".to_string()));
    db_out_exp.create_principal(&"bob".to_string(), &hash("bob_new_pass".to_string()));
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal bob password "bob_pass" do
                            change password bob "bob_new_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string(), sender).await {
        None => assert!(false),
        Some(db_out) => {
            assert_eq!(db_out, db_out_exp);
            assert_eq!(
                receiver.recv().await.unwrap(),
                Entry { status: CREATE_PRINCIPAL, output: None }
            );
            assert_eq!(
                receiver.recv().await.unwrap(),
                Entry { status: RETURNING, output: Some(Immediate("done".to_string())) }
            );
        },
    }
}



