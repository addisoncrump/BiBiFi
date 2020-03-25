use crate::status::Status::FAILED;
use crate::status::{Entry, Status};
use bibifi_database::{Database, Right, Status as DBStatus, Target, Value};
use bibifi_parser::parse;
use bibifi_parser::types::*;
use bibifi_parser::types::{Right as ParserRight, Target as ParserTarget, Value as ParserValue};
use std::collections::HashMap;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub mod status;

#[derive(Clone)]
pub struct BiBiFi {
    sender: UnboundedSender<(String, UnboundedSender<Vec<Entry>>)>,
}

impl BiBiFi {
    pub fn new() -> (
        BiBiFi,
        UnboundedReceiver<(String, UnboundedSender<Vec<Entry>>)>,
    ) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        (BiBiFi { sender }, receiver)
    }

    pub async fn submit(
        &self,
        program: String,
        logback: UnboundedSender<Vec<Entry>>,
    ) -> Result<(), SendError<(String, UnboundedSender<Vec<Entry>>)>> {
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
                    let cmd = &mut program.commands.iter();
                    let mut locals: HashMap<String, Value> = HashMap::new();

                    while let Some(prim) = cmd.next() {
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
                                BiBiFi::set_delegation(&mut database, &program, d)
                            }
                            PrimitiveCommand::DeleteDelegation(d) => {
                                BiBiFi::delete_delegation(&mut database, &program, d)
                            }
                            PrimitiveCommand::DefaultDelegator(p) => {
                                BiBiFi::default_delegator(&mut database, &program, p)
                            }
                        };
                        if res.status == Status::DENIED || res.status == Status::FAILED {
                            return (vec![res], None);
                        }
                        messages.push(res);
                    }
                    match &program.terminator {
                        TerminatorCommand::Exit => {
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
                        TerminatorCommand::Return(e) => {
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
        a: &Assignment,
    ) -> Entry {
        let evaluated = match BiBiFi::evaluate(database, locals, program, &a.expr) {
            Ok(evaluated) => evaluated,
            Err(e) => return e,
        };
        match &a.variable {
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
        ap: &Append,
    ) -> Entry {
        if let Variable::Variable(i) = &ap.variable {
            let evaluated = match BiBiFi::evaluate(database, locals, program, &ap.expr) {
                Ok(evaluated) => evaluated,
                Err(e) => return e,
            };
            match locals.get_mut(&i.name) {
                None => Entry::from(
                    database.append(&program.principal.ident.name, &i.name, &evaluated),
                    Status::APPEND,
                ),
                Some(ref mut value) => {
                    **value = evaluated;
                    Entry {
                        status: Status::APPEND,
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
        la: &Assignment,
    ) -> Entry {
        match &la.variable {
            Variable::Variable(i) => {
                if database.contains(&i.name) || locals.contains_key(&i.name) {
                    Entry {
                        status: Status::FAILED,
                        output: None,
                    }
                } else {
                    let evaluated = match BiBiFi::evaluate(database, locals, program, &la.expr) {
                        Ok(evaluated) => evaluated,
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
                            let modification = |item: Value| {
                                locallocals.insert(i.name.clone(), item);
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

                            if let Some(list) = locals.get(&listi.name).cloned() {
                                match list {
                                    Value::List(list) => {
                                        let modified = list
                                            .iter()
                                            .cloned()
                                            .map(modification)
                                            .collect::<Vec<Result<Value, Entry>>>();
                                        if let Some(bad) =
                                            modified.iter().find(|item| item.is_err())
                                        {
                                            match bad {
                                                Err(e) => e.clone(),
                                                _ => panic!(),
                                            }
                                        } else {
                                            locals.insert(
                                                listi.name.clone(),
                                                Value::List(
                                                    modified
                                                        .iter()
                                                        .map(|item| item.clone().unwrap())
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
                                }
                            } else {
                                match database
                                    .get(&program.principal.ident.name, &listi.name)
                                    .map(|value| value.clone())
                                {
                                    Ok(list) => match list {
                                        Value::List(list) => {
                                            let modified = list
                                                .iter()
                                                .cloned()
                                                .map(modification)
                                                .collect::<Vec<Result<Value, Entry>>>();
                                            if let Some(bad) =
                                                modified.iter().find(|item| item.is_err())
                                            {
                                                match bad {
                                                    Err(e) => e.clone(),
                                                    _ => panic!(),
                                                }
                                            } else {
                                                Entry::from(
                                                    database.set(
                                                        &program.principal.ident.name,
                                                        &listi.name,
                                                        &Value::List(
                                                            modified
                                                                .iter()
                                                                .map(|item| item.clone().unwrap())
                                                                .collect(),
                                                        ),
                                                    ),
                                                    Status::FOREACH,
                                                )
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

    fn set_delegation(database: &mut Database, program: &Program, d: &Delegation) -> Entry {
        Entry::from(
            database.delegate(
                &program.principal.ident.name,
                &match &d.target {
                    ParserTarget::All => Target::All,
                    ParserTarget::Variable(i) => Target::Variable(i.name.clone()),
                },
                &d.delegator.ident.name,
                &match &d.right {
                    ParserRight::Read => Right::Read,
                    ParserRight::Write => Right::Write,
                    ParserRight::Append => Right::Append,
                    ParserRight::Delegate => Right::Delegate,
                },
                &d.delegated.ident.name,
            ),
            Status::SET_DELEGATION,
        )
    }

    fn delete_delegation(database: &mut Database, program: &Program, d: &Delegation) -> Entry {
        Entry::from(
            database.undelegate(
                &program.principal.ident.name,
                &match &d.target {
                    ParserTarget::All => Target::All,
                    ParserTarget::Variable(i) => Target::Variable(i.name.clone()),
                },
                &d.delegator.ident.name,
                &match &d.right {
                    ParserRight::Read => Right::Read,
                    ParserRight::Write => Right::Write,
                    ParserRight::Append => Right::Append,
                    ParserRight::Delegate => Right::Delegate,
                },
                &d.delegated.ident.name,
            ),
            Status::DELETE_DELEGATION,
        )
    }

    fn default_delegator(database: &mut Database, program: &Program, p: &Principal) -> Entry {
        Entry::from(
            database.set_default_delegator(&program.principal.ident.name, &p.ident.name),
            Status::DEFAULT_DELEGATOR,
        )
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
                Variable::Variable(i) => BiBiFi::get_variable(database, locals, program, &i.name),
                Variable::Member(i1, v2) => match v2.as_ref() {
                    Variable::Variable(i2) => {
                        match BiBiFi::get_variable(database, locals, program, &i1.name) {
                            Ok(variable) => match variable {
                                Value::FieldVals(fv) => {
                                    if let Some(s) = fv.get(&i2.name) {
                                        Ok(Value::Immediate(s.clone()))
                                    } else {
                                        Err(Entry {
                                            status: FAILED,
                                            output: None,
                                        })
                                    }
                                }
                                _ => Err(Entry {
                                    status: FAILED,
                                    output: None,
                                }),
                            },
                            Err(e) => Err(e),
                        }
                    }
                    Variable::Member(_, _) => panic!(),
                },
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
#[forbid(unused_must_use)]
mod tests;
