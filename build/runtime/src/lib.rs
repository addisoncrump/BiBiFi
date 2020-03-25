use crate::status::{Entry, Status};
use bibifi_database::Right::Read;
use bibifi_database::{Database, Value};
use bibifi_database::{Right, Target};
use bibifi_parser::parse;
use bibifi_parser::types::Value as ParserValue;
use bibifi_parser::types::*;
use std::borrow::BorrowMut;
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
            if &program.principal.ident.name != "anyone"
                && database.check_principal(&program.principal.ident.name)
            {
                if database.check_pass(&program.principal.ident.name, &program.password) {
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
                            PrimitiveCommand::SetDelegation(d) => BiBiFi::set_delegation(
                                &mut database,
                                &mut locals,
                                &sender,
                                &program,
                                d,
                            ),
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
                                None
                            } else {
                                sender
                                    .send(Entry {
                                        status: Status::EXITING,
                                        output: None,
                                    })
                                    .unwrap();
                                Some(database)
                            }
                        }
                        Command::Return(e) => {
                            let value = BiBiFi::evaluate(&database, &locals, &program, e);
                            match value {
                                Ok(value) => {
                                    sender.send(Entry {
                                        status: Status::RETURNING,
                                        output: Some(value),
                                    });
                                    Some(database)
                                }
                                Err(e) => {
                                    sender.send(e);
                                    None
                                }
                            }
                        }
                        _ => None, // unreachable
                    }
                } else {
                    sender
                        .send(Entry {
                            status: Status::DENIED,
                            output: None,
                        })
                        .unwrap();
                    None
                }
            } else {
                sender
                    .send(Entry {
                        status: Status::FAILED,
                        output: None,
                    })
                    .unwrap();
                None
            }
        } else {
            sender
                .send(Entry {
                    status: Status::FAILED,
                    output: None,
                })
                .unwrap();
            None
        }
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
        if let Variable::Variable(i) = &cp.variable {
            let evaluated = BiBiFi::evaluate(database, locals, program, &cp.expr);
            if database.get(&i.name).is_some() {
                if database.check_right(
                    &Target::Variable(i.name.clone()),
                    &Right::Write,
                    &program.principal.ident.name,
                ) {
                    match evaluated {
                        Ok(value) => {
                            database.set(&i.name, &value);
                            true
                        }
                        Err(e) => {
                            sender.send(e).unwrap();
                            false
                        }
                    }
                } else {
                    sender
                        .send(Entry {
                            status: Status::DENIED,
                            output: None,
                        })
                        .unwrap();
                    false
                }
            } else if let Some(ref mut value) = locals.get(&i.name) {
                match evaluated {
                    Ok(evaluated) => {
                        *value = &evaluated;
                        true
                    }
                    Err(e) => {
                        sender.send(e).unwrap();
                        false
                    }
                }
            } else {
                true
            }
        } else {
            sender
                .send(Entry {
                    status: Status::FAILED,
                    output: None,
                })
                .unwrap();
            false
        }
    }

    fn append(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        sender: &UnboundedSender<Entry>,
        program: &Program,
        cp: &Append,
    ) -> bool {
        if let Variable::Variable(i) = &cp.variable {
            let evaluated = BiBiFi::evaluate(database, locals, program, &cp.expr);
            if let Some(list) = database.get(&i.name) {
                if database.check_right(
                    &Target::Variable(i.name.clone()),
                    &Right::Write,
                    &program.principal.ident.name,
                ) || database.check_right(
                    &Target::Variable(i.name.clone()),
                    &Right::Append,
                    &program.principal.ident.name,
                ) {
                    if let Value::List(list) = list {
                        match evaluated {
                            Ok(value) => match value {
                                Value::Immediate(_) | Value::FieldVals(_) => {
                                    database.set(&i.name, &value);
                                    true
                                }
                                Value::List(evaluated) => {
                                    sender
                                        .send(Entry {
                                            status: Status::FAILED,
                                            output: None,
                                        })
                                        .unwrap();
                                    false
                                }
                            },
                            Err(e) => {
                                sender.send(e).unwrap();
                                false
                            }
                        }
                    } else {
                        sender
                            .send(Entry {
                                status: Status::FAILED,
                                output: None,
                            })
                            .unwrap();
                        false
                    }
                } else {
                    sender
                        .send(Entry {
                            status: Status::DENIED,
                            output: None,
                        })
                        .unwrap();
                    false
                }
            } else if let Some(ref mut value) = locals.get(&i.name) {
                match evaluated {
                    Ok(evaluated) => {
                        *value = &evaluated;
                        true
                    }
                    Err(e) => {
                        sender.send(e).unwrap();
                        false
                    }
                }
            } else {
                true
            }
        } else {
            sender
                .send(Entry {
                    status: Status::FAILED,
                    output: None,
                })
                .unwrap();
            false
        }
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

    fn evaluate(
        database: &Database,
        locals: &HashMap<String, Value>,
        program: &Program,
        expr: &Expr,
    ) -> Result<Value, Entry> {
        match expr {
            Expr::Value(v) => BiBiFi::evaluate_value(database, locals, program, v),
            Expr::EmptyList => Ok(Value::List(Vec::new())),
            Expr::FieldVals(fv) => BiBiFi::evaluate_fieldvals(database, locals, program, fv),
        }
    }

    fn get_variable(
        database: &Database,
        locals: &HashMap<String, Value>,
        program: &Program,
        variable: &String,
        rights: &[Right],
    ) -> Result<Value, Entry> {
        if let Some(value) = database.get(variable) {
            if rights.iter().any(|right| {
                database.check_right(
                    &Target::Variable(variable.clone()),
                    right,
                    &program.principal.ident.name,
                )
            }) {
                Ok(value.clone())
            } else {
                Err(Entry {
                    status: Status::DENIED,
                    output: None,
                })
            }
        } else if let Some(value) = locals.get(variable) {
            Ok(value.clone())
        } else {
            Err(Entry {
                status: Status::FAILED,
                output: None,
            })
        }
    }

    fn modify_variable<F>(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        program: &Program,
        variable: &String,
        rights: &[Right],
        f: F,
    ) -> Option<Entry>
    where
        F: FnOnce(Value) -> Result<Value, Entry>,
    {
        if let Some(value) = database.get(variable) {
            if rights.iter().any(|right| {
                database.check_right(
                    &Target::Variable(variable.clone()),
                    right,
                    &program.principal.ident.name,
                )
            }) {
                match f(value.clone()) {
                    Ok(value) => {
                        database.set(variable, &value);
                        None
                    }
                    Err(e) => Some(e),
                }
            } else {
                Some(Entry {
                    status: Status::DENIED,
                    output: None,
                })
            }
        } else if let Some(value) = locals.get(variable) {
            match f(value.clone()) {
                Ok(value) => {
                    database.set(variable, &value);
                    None
                }
                Err(e) => Some(e),
            }
        } else {
            Some(Entry {
                status: Status::FAILED,
                output: None,
            })
        }
    }

    fn evaluate_value(
        database: &Database,
        locals: &HashMap<String, Value>,
        program: &Program,
        value: &ParserValue,
    ) -> Result<Value, Entry> {
        match value {
            ParserValue::Variable(v) => match v {
                Variable::Variable(i) | Variable::Member(i, _) => {
                    BiBiFi::get_variable(database, locals, program, &i.name, &[Read])
                }
            },
            ParserValue::String(s) => Ok(Value::Immediate(s.clone())),
        }
    }

    fn evaluate_fieldvals(
        database: &Database,
        locals: &HashMap<String, Value>,
        program: &Program,
        value: &Vec<Assignment>,
    ) -> Result<Value, Entry> {
        let mut map = HashMap::new();
        for a in value {
            match &a.variable {
                Variable::Variable(i) => {
                    if map.contains_key(&i.name) {
                        return Err(Entry {
                            // duplicate entry
                            status: Status::FAILED,
                            output: None,
                        });
                    }
                    map.insert(
                        i.name.clone(),
                        match &a.expr {
                            Expr::Value(value) => {
                                match BiBiFi::evaluate_value(database, locals, program, value) {
                                    Ok(value) => match value {
                                        Value::Immediate(i) => i,
                                        Value::List(_) | Value::FieldVals(_) => {
                                            return Err(Entry {
                                                status: Status::FAILED,
                                                output: None,
                                            })
                                        }
                                    },
                                    Err(e) => return Err(e),
                                }
                            }
                            Expr::EmptyList | Expr::FieldVals(_) => {
                                return Err(Entry {
                                    status: Status::FAILED,
                                    output: None,
                                })
                            }
                        },
                    );
                }
                Variable::Member(_, _) => {
                    return Err(Entry {
                        status: Status::FAILED,
                        output: None,
                    })
                }
            }
        }
        Ok(Value::FieldVals(map))
    }
}

#[cfg(test)]
mod tests;
