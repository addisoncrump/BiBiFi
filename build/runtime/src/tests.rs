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
