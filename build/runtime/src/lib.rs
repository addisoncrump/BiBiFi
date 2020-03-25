use crate::status::{Entry, Status};
use bibifi_database::{Database, Status as DBStatus, Value};
use bibifi_parser::parse;
use bibifi_parser::types::Value as ParserValue;
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
        mut receiver: UnboundedReceiver<(String, UnboundedSender<Vec<Entry>>)>,
    ) {
        let mut database = Database::new(hash);
        while let Some((program, sender)) = receiver.recv().await {
            let (messages, returned) = BiBiFi::run_program(database.clone(), program).await;
            sender.send(messages).unwrap();
            database = match returned {
                None => database,
                Some(returned) => returned,
            };
        }
    }

    // segmented out for testing :)
    async fn run_program(
        mut database: Database,
        program: String,
    ) -> (Vec<Entry>, Option<Database>) {
        let program = parse(program);
        let mut messages = Vec::new();
        if let Ok(program) = program {
            match database.check_pass(&program.principal.ident.name, &program.password) {
                DBStatus::SUCCESS => {
                    let mut cmd = &program.command;
                    let mut locals: HashMap<String, Value> = HashMap::new();

                    while let Command::Chain(prim, next) = cmd {
                        cmd = &*next;
                        let res = match prim {
                            PrimitiveCommand::CreatePrincipal(cp) => {
                                BiBiFi::create_principal(&mut database, &program, cp)
                            }
                            PrimitiveCommand::ChangePassword(cp) => {
                                BiBiFi::change_password(&mut database, &program, cp)
                            }
                            PrimitiveCommand::Assignment(a) => {
                                BiBiFi::assignment(&mut database, &mut locals, &program, a)
                            }
                            PrimitiveCommand::Append(a) => {
                                BiBiFi::append(&mut database, &mut locals, &program, a)
                            }
                            PrimitiveCommand::LocalAssignment(a) => {
                                BiBiFi::local_assignment(&mut database, &mut locals, &program, a)
                            }
                            PrimitiveCommand::ForEach(fe) => {
                                BiBiFi::for_each(&mut database, &mut locals, &program, fe)
                            }
                            PrimitiveCommand::SetDelegation(d) => {
                                BiBiFi::set_delegation(&mut database, &mut locals, &program, d)
                            }
                            PrimitiveCommand::DeleteDelegation(d) => {
                                BiBiFi::delete_delegation(&mut database, &mut locals, &program, d)
                            }
                            PrimitiveCommand::DefaultDelegator(p) => {
                                BiBiFi::default_delegator(&mut database, &mut locals, &program, p)
                            }
                        };
                        if res.status == Status::DENIED || res.status == Status::FAILED {
                            return (vec![res], None);
                        }
                        messages.push(res);
                    }
                    match cmd {
                        Command::Exit => {
                            if &program.principal.ident.name != "admin" {
                                (
                                    vec![Entry {
                                        status: Status::DENIED,
                                        output: None,
                                    }],
                                    None,
                                )
                            } else {
                                messages.push(Entry {
                                    status: Status::EXITING,
                                    output: None,
                                });
                                (messages, Some(database))
                            }
                        }
                        Command::Return(e) => {
                            let value = BiBiFi::evaluate(&database, &locals, &program, e);
                            match value {
                                Ok(value) => {
                                    messages.push(Entry {
                                        status: Status::RETURNING,
                                        output: Some(value),
                                    });
                                    (messages, Some(database))
                                }
                                Err(e) => (vec![e], None),
                            }
                        }
                        _ => panic!(),
                    }
                }
                DBStatus::DENIED => (
                    vec![Entry {
                        status: Status::DENIED,
                        output: None,
                    }],
                    None,
                ),
                DBStatus::FAILED => (
                    vec![Entry {
                        status: Status::FAILED,
                        output: None,
                    }],
                    None,
                ),
            }
        } else {
            (
                vec![Entry {
                    status: Status::FAILED,
                    output: None,
                }],
                None,
            )
        }
    }

    fn change_password(database: &mut Database, program: &Program, cp: &ChangePassword) -> Entry {
        Entry::from(
            database.change_password(
                &program.principal.ident.name,
                &cp.principal.ident.name,
                &cp.password,
            ),
            Status::CHANGE_PASSWORD,
        )
    }

    fn create_principal(database: &mut Database, program: &Program, cp: &CreatePrincipal) -> Entry {
        Entry::from(
            database.create_principal(
                &program.principal.ident.name,
                &cp.principal.ident.name,
                &cp.password,
            ),
            Status::CREATE_PRINCIPAL,
        )
    }

    fn assignment(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        program: &Program,
        cp: &Assignment,
    ) -> Entry {
        let evaluated = match BiBiFi::evaluate(database, locals, program, &cp.expr) {
            Ok(evaluated) => evaluated,
            Err(e) => return e,
        };
        match &cp.variable {
            Variable::Variable(i) => {
                if locals.contains_key(&i.name) {
                    locals.insert(i.name.clone(), evaluated);
                    Entry {
                        status: Status::SET,
                        output: None,
                    }
                } else {
                    Entry::from(
                        database.set(&program.principal.ident.name, &i.name, &evaluated),
                        Status::SET,
                    )
                }
            }
            Variable::Member(i1, i2) => match evaluated {
                Value::Immediate(s) => {
                    if let Variable::Variable(i) = i2.as_ref() {
                        if let Some(ref mut value) = locals.get_mut(&i1.name) {
                            if let Value::FieldVals(map) = value {
                                if let Some(existing) = map.get_mut(&i.name) {
                                    *existing = s;
                                    Entry {
                                        status: Status::SET,
                                        output: None,
                                    }
                                } else {
                                    Entry {
                                        status: Status::FAILED,
                                        output: None,
                                    }
                                }
                            } else {
                                Entry {
                                    status: Status::FAILED,
                                    output: None,
                                }
                            }
                        } else {
                            Entry::from(
                                database.set_member(
                                    &program.principal.ident.name,
                                    &i1.name,
                                    &i.name,
                                    &s,
                                ),
                                Status::SET,
                            )
                        }
                    } else {
                        Entry {
                            status: Status::FAILED,
                            output: None,
                        }
                    }
                }
                _ => Entry {
                    status: Status::FAILED,
                    output: None,
                },
            },
        }
    }

    fn append(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        program: &Program,
        cp: &Append,
    ) -> Entry {
        if let Variable::Variable(i) = &cp.variable {
            let evaluated = match BiBiFi::evaluate(database, locals, program, &cp.expr) {
                Ok(evaluated) => match evaluated {
                    Value::Immediate(_) | Value::FieldVals(_) => evaluated,
                    Value::List(_) => {
                        return Entry {
                            status: Status::FAILED,
                            output: None,
                        }
                    }
                },
                Err(e) => return e,
            };
            match locals.get_mut(&i.name) {
                None => Entry::from(
                    database.append(&program.principal.ident.name, &i.name, &evaluated),
                    Status::SET,
                ),
                Some(ref mut value) => {
                    **value = evaluated;
                    Entry {
                        status: Status::SET,
                        output: None,
                    }
                }
            }
        } else {
            Entry {
                status: Status::FAILED,
                output: None,
            }
        }
    }

    fn local_assignment(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        program: &Program,
        cp: &Assignment,
    ) -> Entry {
        match &cp.variable {
            Variable::Variable(i) => {
                if database.contains(&i.name) || locals.contains_key(&i.name) {
                    Entry {
                        status: Status::FAILED,
                        output: None,
                    }
                } else {
                    let evaluated = match BiBiFi::evaluate(database, locals, program, &cp.expr) {
                        Ok(evaluated) => match evaluated {
                            Value::Immediate(_) | Value::FieldVals(_) => evaluated,
                            Value::List(_) => {
                                return Entry {
                                    status: Status::FAILED,
                                    output: None,
                                }
                            }
                        },
                        Err(e) => return e,
                    };
                    locals.insert(i.name.clone(), evaluated);
                    Entry {
                        status: Status::LOCAL,
                        output: None,
                    }
                }
            }
            Variable::Member(_, _) => Entry {
                status: Status::FAILED,
                output: None,
            },
        }
    }

    fn for_each(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        program: &Program,
        fe: &ForEach,
    ) -> Entry {
        match &fe.value {
            Variable::Variable(i) => {
                if locals.contains_key(&i.name) || database.contains(&i.name) {
                    Entry {
                        status: Status::FAILED,
                        output: None,
                    }
                } else {
                    match &fe.list {
                        Variable::Variable(listi) => {
                            let mut locallocals = locals.clone();
                            let modification = |item: &Value| {
                                locallocals.insert(i.name.clone(), item.clone());
                                let res = match BiBiFi::evaluate(
                                    database,
                                    &locallocals,
                                    program,
                                    &fe.expr,
                                ) {
                                    Ok(v) => Ok(v),
                                    Err(e) => Err(e),
                                };
                                res
                            };
                            locals.remove(&i.name).unwrap();

                            if let Some(list) = locals.get(&listi.name) {
                                match list {
                                    Value::List(list) => {
                                        let mut modified = list.iter().map(modification);
                                        if let Some(bad) = modified.find(|item| item.is_err()) {
                                            match bad {
                                                Ok(_) => panic!(),
                                                Err(e) => e,
                                            }
                                        } else {
                                            locals.insert(
                                                listi.name.clone(),
                                                Value::List(
                                                    modified.map(|item| item.unwrap()).collect(),
                                                ),
                                            );
                                            Entry {
                                                status: Status::FOREACH,
                                                output: None,
                                            }
                                        }
                                    }
                                    _ => Entry {
                                        status: Status::FAILED,
                                        output: None,
                                    },
                                }
                            } else {
                                match database.get(&program.principal.ident.name, &listi.name) {
                                    Ok(list) => match list {
                                        Value::List(list) => {
                                            let mut modified = list.iter().map(modification);
                                            if let Some(bad) = modified.find(|item| item.is_err()) {
                                                match bad {
                                                    Ok(_) => panic!(),
                                                    Err(e) => e,
                                                }
                                            } else {
                                                database.set(
                                                    &program.principal.ident.name,
                                                    &listi.name,
                                                    &Value::List(
                                                        modified
                                                            .map(|item| item.unwrap())
                                                            .collect(),
                                                    ),
                                                );
                                                Entry {
                                                    status: Status::FOREACH,
                                                    output: None,
                                                }
                                            }
                                        }
                                        _ => Entry {
                                            status: Status::FAILED,
                                            output: None,
                                        },
                                    },
                                    Err(e) => Entry::from(e, Status::FAILED),
                                }
                            }
                        }
                        Variable::Member(_, _) => Entry {
                            status: Status::FAILED,
                            output: None,
                        },
                    }
                }
            }
            Variable::Member(_, _) => Entry {
                status: Status::FAILED,
                output: None,
            },
        }
    }

    fn set_delegation(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        program: &Program,
        d: &Delegation,
    ) -> Entry {
        Entry {
            status: Status::FAILED,
            output: None,
        }
    }

    fn delete_delegation(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        program: &Program,
        d: &Delegation,
    ) -> Entry {
        Entry {
            status: Status::FAILED,
            output: None,
        }
    }

    fn default_delegator(
        database: &mut Database,
        locals: &mut HashMap<String, Value>,
        program: &Program,
        p: &Principal,
    ) -> Entry {
        Entry {
            status: Status::FAILED,
            output: None,
        }
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
    ) -> Result<Value, Entry> {
        match locals.get(variable) {
            None => match database.get(&program.principal.ident.name, variable) {
                Ok(value) => Ok(value.clone()),
                Err(DBStatus::DENIED) => Err(Entry {
                    status: Status::DENIED,
                    output: None,
                }),
                Err(DBStatus::FAILED) => Err(Entry {
                    status: Status::FAILED,
                    output: None,
                }),
                _ => panic!(),
            },
            Some(value) => Ok(value.clone()),
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
                    BiBiFi::get_variable(database, locals, program, &i.name)
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
