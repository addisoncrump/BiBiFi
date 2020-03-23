use crate::status::{Entry, Status};
use bibifi_database::{Database, Value};
use bibifi_parser::parse;
use bibifi_parser::types::{Command, PrimitiveCommand, Variable};
use std::collections::HashMap;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub mod status;

#[derive(Clone)]
pub struct BiBiFi {
    sender: UnboundedSender<(String, UnboundedSender<Entry>)>,
}

impl BiBiFi {
    pub fn new() -> (BiBiFi, UnboundedReceiver<(String, UnboundedSender<Entry>)>) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        (BiBiFi { sender }, receiver)
    }

    pub async fn submit(
        &self,
        program: String,
        logback: UnboundedSender<Entry>,
    ) -> Result<(), SendError<(String, UnboundedSender<Entry>)>> {
        self.sender.send((program, logback))
    }

    // single "thread" per task
    pub async fn run(
        hash: [u8; 32],
        mut receiver: UnboundedReceiver<(String, UnboundedSender<Entry>)>,
    ) {
        let mut database = Database::new(hash);
        while let Some((program, sender)) = receiver.recv().await {
            database = match BiBiFi::run_program(database.clone(), program, sender).await {
                None => database,
                Some(new_database) => new_database,
            }
        }
    }

    // segmented out for testing :)
    async fn run_program(
        mut database: Database,
        program: String,
        sender: UnboundedSender<Entry>,
    ) -> Option<Database> {
        let program = parse(program);
        if let Ok(program) = program {
            if database.check_principal(&program.principal.ident.name)
                && database.check_pass(&program.principal.ident.name, &program.password)
            {
                let mut cmd = program.command;
                let mut locals: HashMap<String, Value> = HashMap::new();

                while let Command::Chain(prim, next) = cmd {
                    cmd = *next;
                    match prim {
                        PrimitiveCommand::CreatePrincipal(cp) => {
                            if database.check_principal(&cp.principal.ident.name) {
                                sender
                                    .send(Entry {
                                        status: Status::FAILED,
                                        output: None,
                                    })
                                    .unwrap();
                                return None;
                            } else if &program.principal.ident.name != "admin" {
                                sender
                                    .send(Entry {
                                        status: Status::DENIED,
                                        output: None,
                                    })
                                    .unwrap();
                                return None;
                            }
                            database.create_principal(&cp.principal.ident.name, &cp.password);
                            sender
                                .send(Entry {
                                    status: Status::CREATE_PRINCIPAL,
                                    output: None,
                                })
                                .unwrap();
                        }
                        PrimitiveCommand::ChangePassword(cp) => {
                            if database.check_principal(&cp.principal.ident.name) {
                                sender
                                    .send(Entry {
                                        status: Status::FAILED,
                                        output: None,
                                    })
                                    .unwrap();
                                return None;
                            } else if &program.principal.ident.name != "admin"
                                && &program.principal.ident.name != &cp.principal.ident.name
                            {
                                sender
                                    .send(Entry {
                                        status: Status::DENIED,
                                        output: None,
                                    })
                                    .unwrap();
                                return None;
                            }
                            database.change_password(&cp.principal.ident.name, &cp.password);
                            sender
                                .send(Entry {
                                    status: Status::CHANGE_PASSWORD,
                                    output: None,
                                })
                                .unwrap();
                        }
                        PrimitiveCommand::Assignment(a) => {
                            let name = match a.variable {
                                Variable::Variable(i) | Variable::Member(i, _) => &i.name,
                            };
                        }
                        PrimitiveCommand::Append(_) => {}
                        PrimitiveCommand::LocalAssignment(_) => {}
                        PrimitiveCommand::ForEach(_) => {}
                        PrimitiveCommand::SetDelegation(_) => {}
                        PrimitiveCommand::DeleteDelegation(_) => {}
                        PrimitiveCommand::DefaultDelegator(_) => {}
                    }
                }
                match cmd {
                    Command::Exit => {
                        if &program.principal.ident.name != "admin" {
                            sender
                                .send(Entry {
                                    status: Status::DENIED,
                                    output: None,
                                })
                                .unwrap();
                        } else {
                            sender
                                .send(Entry {
                                    status: Status::EXITING,
                                    output: None,
                                })
                                .unwrap();
                        }
                    }
                    Command::Return(_) => {}
                    _ => {}
                }
            }
        }
        sender
            .send(Entry {
                status: Status::FAILED,
                output: None,
            })
            .unwrap();
        None
    }
}
