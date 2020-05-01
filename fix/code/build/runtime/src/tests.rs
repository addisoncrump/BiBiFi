use super::*;
use crate::status::Status::*;
use bibifi_database::Database;
use bibifi_database::Value;
use bibifi_database::Value::Immediate;
use bibifi_util::hash;
use tokio::sync::mpsc::unbounded_channel;

#[tokio::test]
async fn example() {
    let db = Database::new(hash("admin".to_string()));
    let program = r#"as principal admin password "admin" do
    exit
    ***"#;
    match BiBiFi::run_program(db.clone(), program.to_string()).await {
        (out_message, Some(db1)) => {
            assert_eq!(db, db1);
            assert_eq!(
                vec![Entry {
                    status: EXITING,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// parsing errors take precedence over security violation
// test with invalid syntax and incorrect password should return FAILED
#[tokio::test]
async fn t1_parse_err_vs_sec_fail() {
    let db_in = Database::new(hash("admin_pass".to_string()));
    let program = r#"as principal admin password "admin" do exit
    ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: FAILED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// acting principal has to be there in database
// test with non-existing acting principal should return FAILED
#[tokio::test]
async fn t2_non_existing_acting_principal() {
    let db_in = Database::new(hash("admin_pass".to_string()));
    let program = r#"as principal bob password "admin_pass" do
                            exit
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: FAILED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// acting principal cannot be anyone
// test with anyone acting principal should return DENIED
#[tokio::test]
async fn t3_anyone_acting_principal() {
    let db_in = Database::new(hash("admin_pass".to_string()));
    let program = r#"as principal anyone password "admin_pass" do
                            exit
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: DENIED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// acting principal should have correct password
// test with incorrect password should return DENIED
#[tokio::test]
async fn t4_acting_p_inc_pass() {
    let db_in = Database::new(hash("admin_pass".to_string()));
    let program = r#"as principal admin password "admin" do
                            exit
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: DENIED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// only admin can give exit
// test with non-admin giving exit command should return DENIED
#[tokio::test]
async fn t5_non_admin_exit_cmd() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_in.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string()),
        ),
        DBStatus::SUCCESS
    );
    let program = r#"as principal bob password "bob_pass" do
                            exit
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: DENIED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// return command should return expr
// test with return cmd should return expr
#[tokio::test]
async fn t6_non_admin_exit_cmd() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_in.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string()),
        ),
        DBStatus::SUCCESS
    );
    let program = r#"as principal bob password "bob_pass" do
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, Some(db_out)) => {
            assert_eq!(db_out, db_in);
            assert_eq!(
                vec![Entry {
                    status: RETURNING,
                    output: Some(Value::Immediate("done".to_string()))
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// create principal should create principal
#[tokio::test]
async fn t7_create_principal() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    let mut db_out_exp = db_in.clone();
    assert_eq!(
        db_out_exp.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string()),
        ),
        DBStatus::SUCCESS
    );
    let program = r#"as principal admin password "admin_pass" do
                            create principal bob "bob_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, Some(db_out)) => {
            assert_eq!(db_out, db_out_exp);
            assert_eq!(
                vec![
                    Entry {
                        status: CREATE_PRINCIPAL,
                        output: None
                    },
                    Entry {
                        status: RETURNING,
                        output: Some(Value::Immediate("done".to_string()))
                    }
                ],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// only admin can create principal
// test with non-admin creating principal
#[tokio::test]
async fn t8_non_admin_create_principal() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_in.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string()),
        ),
        DBStatus::SUCCESS
    );
    let program = r#"as principal bob password "bob_pass" do
                            create principal alice "alice_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: DENIED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// cannot create existing principal
// test with recreating principal
#[tokio::test]
async fn t9_recreate_principal() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_in.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string()),
        ),
        DBStatus::SUCCESS
    );
    let program = r#"as principal admin password "admin_pass" do
                            create principal anyone "anyone_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: FAILED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// change password
#[tokio::test]
async fn t10_change_password() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_in.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string()),
        ),
        DBStatus::SUCCESS
    );
    let mut db_out_exp = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_out_exp.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_new_pass".to_string()),
        ),
        DBStatus::SUCCESS
    );
    let program = r#"as principal bob password "bob_pass" do
                            change password bob "bob_new_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, Some(db_out)) => {
            assert_eq!(db_out, db_out_exp);
            assert_eq!(
                vec![
                    Entry {
                        status: CHANGE_PASSWORD,
                        output: None
                    },
                    Entry {
                        status: RETURNING,
                        output: Some(Value::Immediate("done".to_string()))
                    }
                ],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// admin change password
#[tokio::test]
async fn t11_admin_change_password() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    let mut db_out_exp = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_out_exp.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_new_pass".to_string()),
        ),
        DBStatus::SUCCESS
    );
    let program = r#"as principal admin password "admin_pass" do
                            create principal bob "bob_pass"
                            change password bob "bob_new_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, Some(db_out)) => {
            assert_eq!(db_out, db_out_exp);
            assert_eq!(
                vec![
                    Entry {
                        status: CREATE_PRINCIPAL,
                        output: None
                    },
                    Entry {
                        status: CHANGE_PASSWORD,
                        output: None
                    },
                    Entry {
                        status: RETURNING,
                        output: Some(Value::Immediate("done".to_string()))
                    }
                ],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// principal has to exist to change password
// test to change password fr non-existing user
#[tokio::test]
async fn t12_non_exist_pric_change_password() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    let mut db_out_exp = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_out_exp.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_new_pass".to_string())
        ),
        DBStatus::SUCCESS
    );
    let program = r#"as principal admin password "admin_pass" do
                            create principal bob "bob_pass"
                            change password alice "bob_new_pass"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: FAILED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// cannot set without permissions
// test to set variable without permissions
#[tokio::test]
async fn t13_set_without_permission() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_in.set(
            &"admin".to_string(),
            &"my_var".to_string(),
            &Value::Immediate("wolla".to_string())
        ),
        DBStatus::SUCCESS
    );
    assert_eq!(
        db_in.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string())
        ),
        DBStatus::SUCCESS
    );
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal bob password "bob_pass" do
                            set my_var = "hi"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: DENIED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// first issues of expr need to be resolved
// test with denied in lhs ans failed in rhs
#[tokio::test]
async fn t14_lhs_rhs_pref() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_in.set(
            &"admin".to_string(),
            &"my_var".to_string(),
            &Value::Immediate("wolla".to_string())
        ),
        DBStatus::SUCCESS
    );
    assert_eq!(
        db_in.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string())
        ),
        DBStatus::SUCCESS
    );
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal bob password "bob_pass" do
                            set my_var = y
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, None) => {
            assert_eq!(
                vec![Entry {
                    status: FAILED,
                    output: None
                }],
                out_message
            );
        }
        _ => assert!(false),
    }
}

// append permission is enough fr append
#[tokio::test]
async fn t15_append() {
    let mut db_in = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_in.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string())
        ),
        DBStatus::SUCCESS
    );
    assert_eq!(
        db_in.set(
            &"admin".to_string(),
            &"my_var".to_string(),
            &Value::List(vec![Value::Immediate("wolla".to_string())])
        ),
        DBStatus::SUCCESS
    );
    assert_eq!(
        db_in.delegate(
            &"admin".to_string(),
            &Target::Variable("my_var".to_string()),
            &"admin".to_string(),
            &Right::Append,
            &"bob".to_string()
        ),
        DBStatus::SUCCESS
    );
    let mut db_out_exp = Database::new(hash("admin_pass".to_string()));
    assert_eq!(
        db_out_exp.create_principal(
            &"admin".to_string(),
            &"bob".to_string(),
            &hash("bob_pass".to_string())
        ),
        DBStatus::SUCCESS
    );
    assert_eq!(
        db_out_exp.set(
            &"admin".to_string(),
            &"my_var".to_string(),
            &Value::List(vec![
                Value::Immediate("wolla".to_string()),
                Value::Immediate("added".to_string())
            ])
        ),
        DBStatus::SUCCESS
    );
    assert_eq!(
        db_out_exp.delegate(
            &"admin".to_string(),
            &Target::Variable("my_var".to_string()),
            &"admin".to_string(),
            &Right::Append,
            &"bob".to_string()
        ),
        DBStatus::SUCCESS
    );
    let (sender, mut receiver) = unbounded_channel::<Entry>();
    let program = r#"as principal bob password "bob_pass" do
                            append to my_var with "added"
                            return "done"
                            ***"#;
    match BiBiFi::run_program(db_in.clone(), program.to_string()).await {
        (out_message, Some(db_out)) => {
            assert_eq!(db_out, db_out_exp);
            assert_eq!(
                vec![
                    Entry {
                        status: APPEND,
                        output: None
                    },
                    Entry {
                        status: RETURNING,
                        output: Some(Value::Immediate("done".to_string()))
                    }
                ],
                out_message
            );
        }
        _ => assert!(false),
    }
}
