use crate::status::{Entry, Status};
use bibifi_database::{Database, Value};
use bibifi_parser::parse;
use bibifi_parser::types::*;
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
                let mut cmd = &program.command;
                let mut locals: HashMap<String, Value> = HashMap::new();

                while let Command::Chain(prim, next) = cmd {
                    cmd = &*next;
                    match match prim {
                        PrimitiveCommand::CreatePrincipal(cp) => BiBiFi::create_principal(
                            &mut database,
                            &mut locals,
                            &sender,
                            &program,
                            cp,
                        ),
                        PrimitiveCommand::ChangePassword(cp) => BiBiFi::change_password(
                            &mut database,
                            &mut locals,
                            &sender,
                            &program,
                            cp,
                        ),
                        PrimitiveCommand::Assignment(a) => {
                            BiBiFi::assignment(&mut database, &mut locals, &sender, &program, a)
                        }
                        PrimitiveCommand::Append(a) => {
                            BiBiFi::append(&mut database, &mut locals, &sender, &program, a)
                        }
                        PrimitiveCommand::LocalAssignment(a) => BiBiFi::local_assignment(
                            &mut database,
                            &mut locals,
                            &sender,
                            &program,
                            a,
                        ),
                        PrimitiveCommand::ForEach(fe) => {
                            BiBiFi::for_each(&mut database, &mut locals, &sender, &program, fe)
                        }
                        PrimitiveCommand::SetDelegation(d) => {
                            BiBiFi::set_delegation(&mut database, &mut locals, &sender, &program, d)
                        }
                        PrimitiveCommand::DeleteDelegation(d) => BiBiFi::delete_delegation(
                            &mut database,
                            &mut locals,
                            &sender,
                            &program,
                            d,
                        ),
                        PrimitiveCommand::DefaultDelegator(p) => BiBiFi::default_delegator(
                            &mut database,
                            &mut locals,
                            &sender,
                            &program,
                            p,
                        ),
                    } {
                        false => return None,
                        _ => {}
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

    fn change_password(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        cp: &ChangePassword,
    ) -> bool {
        if database.check_principal(&cp.principal.ident.name) {
            sender
                .send(Entry {
                    status: Status::FAILED,
                    output: None,
                })
                .unwrap();
            false
        } else if &program.principal.ident.name != "admin"
            && &program.principal.ident.name != &cp.principal.ident.name
        {
            sender
                .send(Entry {
                    status: Status::DENIED,
                    output: None,
                })
                .unwrap();
            false
        } else {
            database.change_password(&cp.principal.ident.name, &cp.password);
            sender
                .send(Entry {
                    status: Status::CHANGE_PASSWORD,
                    output: None,
                })
                .unwrap();
            true
        }
    }

    fn create_principal(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        cp: &CreatePrincipal,
    ) -> bool {
        if database.check_principal(&cp.principal.ident.name) {
            sender
                .send(Entry {
                    status: Status::FAILED,
                    output: None,
                })
                .unwrap();
            false
        } else if &program.principal.ident.name != "admin" {
            sender
                .send(Entry {
                    status: Status::DENIED,
                    output: None,
                })
                .unwrap();
            false
        } else {
            database.create_principal(&cp.principal.ident.name, &cp.password);
            sender
                .send(Entry {
                    status: Status::CREATE_PRINCIPAL,
                    output: None,
                })
                .unwrap();
            true
        }
    }

    fn assignment(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        cp: &Assignment,
    ) -> bool {
        false
    }

    fn append(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        cp: &Append,
    ) -> bool {
        false
    }

    fn local_assignment(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        cp: &Assignment,
    ) -> bool {
        false
    }

    fn for_each(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        fe: &ForEach,
    ) -> bool {
        false
    }

    fn set_delegation(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        d: &Delegation,
    ) -> bool {
        false
    }

    fn delete_delegation(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        d: &Delegation,
    ) -> bool {
        false
    }

    fn default_delegator(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        p: &Principal,
    ) -> bool {
        false
    }
}
