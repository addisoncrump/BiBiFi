use super::*;
use std::error::Error;

#[test]
fn basic_full_1() -> Result<(), Box<dyn Error>> { // Basic test covering full grammer
    let program = program_parser::program(
        r#"as principal bob password "lmao" do
              create principal jack "hammer"
              change password bob "bits"  // please ignore me
              set x = "done"
              set a = []
              //set b = {p = x, q = "that", r = "has"} //dont know how to handle vectors
              //set d = b.q
              //set e = {k = d, l = b.r}
              append to x with "it"
              local y = "temp_t"
              foreach x in y replacewith "temp_i"
              set delegation all jack read -> bob
              delete delegation x jack read -> bob
              default delegator = jack
              return "SUCCESS"
       ***"#,
    )?;

    assert_eq!(
        program,
        Program {
            principal: Principal { ident: Identifier { name: "bob".to_string() } },
            password: "lmao".to_string(),
            command: Command::Chain(
              PrimitiveCommand::CreatePrincipal(CreatePrincipal 
                                    {principal: Principal {ident: Identifier {name: "jack".to_string()}}, 
                                     password: "hammer".to_string()}),
              Box::new(Command::Chain(
                PrimitiveCommand::ChangePassword(ChangePassword 
                                    {principal: Principal {ident: Identifier {name: "bob".to_string()}}, 
                                     password: "bits".to_string()}),
                Box::new(Command::Chain(
                  PrimitiveCommand::Assignment(Assignment 
                                    {variable: Variable::Variable (Identifier {name: "x".to_string()}), 
                                     expr: Expr::Value(Value::String("done".to_string()))}),
                  Box::new(Command::Chain(
                    PrimitiveCommand::Assignment(Assignment 
                                    {variable: Variable::Variable (Identifier {name: "a".to_string()}), 
                                     expr: Expr::EmptyList}),
                    Box::new(Command::Chain(
                      PrimitiveCommand::Append(Append 
                                    {variable: Variable::Variable (Identifier {name: "x".to_string()}), 
                                     expr: Expr::Value(Value::String ("it".to_string()))}),
                      Box::new(Command::Chain(
                        PrimitiveCommand::LocalAssignment(Assignment 
                                    {variable: Variable::Variable (Identifier {name: "y".to_string()}), 
                                     expr: Expr::Value(Value::String ("temp_t".to_string()))}),
                        Box::new(Command::Chain(
                          PrimitiveCommand::ForEach(ForEach 
                                    {value: Variable::Variable (Identifier {name: "x".to_string()}), 
                                     list: Variable::Variable (Identifier {name: "y".to_string()}), 
                                     expr: Expr::Value(Value::String ("temp_i".to_string()))}),
                          Box::new(Command::Chain(
                            PrimitiveCommand::SetDelegation(Delegation 
                                    {target: Target::All, 
                                     delegator: Principal {ident: Identifier {name: "jack".to_string()}}, 
                                     right: Right::Read,
                                     delegated: Principal {ident: Identifier {name: "bob".to_string()}}}), 
                            Box::new(Command::Chain(
                              PrimitiveCommand::DeleteDelegation(Delegation 
                                    {target: Target::Variable (Variable::Variable (Identifier {name: "x".to_string()})), 
                                     delegator: Principal {ident: Identifier {name: "jack".to_string()}}, 
                                     right: Right::Read,
                                     delegated: Principal {ident: Identifier {name: "bob".to_string()}}}), 
                              Box::new(Command::Chain(
                                PrimitiveCommand::DefaultDelegator(Principal {ident: Identifier {name: "jack".to_string()}}),
                                Box::new(Command::Return (Expr::Value (Value::String ("SUCCESS".to_string())))) 
                              ))
                            ))
                          ))
                        ))
                      ))
                    ))
                  ))
                ))
              ))
            )
        }
    );

    Ok(())
}

#[test]
fn basic() -> Result<(), Box<dyn Error>> {
    let program = program_parser::program(
        r#"as principal bob password "lmao" do
            exit
       ***"#,
    )?;

    assert_eq!(
        program,
        Program {
            principal: Principal {
                ident: Identifier {
                    name: "bob".to_string()
                }
            },
            password: "lmao".to_string(),
            command: Command::Exit
        }
    );

    Ok(())
}

#[test]
fn basic_1() -> Result<(), Box<dyn Error>> {
    let program = program_parser::program(
        r#"as principal alice password "alices_password" do
              return msg
       ***"#,
    )?;

    assert_eq!(
        program,
        Program {
            principal: Principal {
                ident: Identifier {
                    name: "alice".to_string()
                }
            },
            password: "alices_password".to_string(),
            command: Command::Return(Expr::Value(Value::Variable(Variable::Variable(Identifier{name: "msg".to_string()}))))
        }
    );

    Ok(())
}


//#[test]
//fn basic_2() {
//    assert!(program_parser::program(
//        r#"as principal aliiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiice password "alices_password" do
//              return msg
//       ***"#,
//    ).is_err());}


#[test]
fn fail_1() {
    assert!(program_parser::program(
        r#"as principal alice password "alices_password" do
              return msg
        ***
           as principal alice password "alices_password" do
              return msg
       ***"#,
    ).is_err());
}

#[test]
fn example() -> Result<(), Box<dyn Error>> {
    let program = program_parser::program(
        r#"as principal admin password "admin" do
create principal alice "alices_password"
set msg = "Hi Alice. Good luck in Build-it, Break-it, Fix-it!"
set delegation msg admin read -> alice
return "success"
***"#,
    )?;

    assert_eq!(
        program,
        Program {
            principal: Principal {
                ident: Identifier {
                    name: "admin".to_string()
                }
            },
            password: "admin".to_string(),
            command: Command::Chain(
                PrimitiveCommand::CreatePrincipal(CreatePrincipal {
                    principal: Principal {
                        ident: Identifier {
                            name: "alice".to_string()
                        },
                    },
                    password: "alices_password".to_string()
                }),
                Box::new(Command::Chain(
                    PrimitiveCommand::Assignment(Assignment {
                        variable: Variable::Variable(Identifier {
                            name: "msg".to_string()
                        }),
                        expr: Expr::Value(Value::String(
                            "Hi Alice. Good luck in Build-it, Break-it, Fix-it!".to_string()
                        ))
                    }),
                    Box::new(Command::Chain(
                        PrimitiveCommand::SetDelegation(Delegation {
                            target: Target::Variable(Variable::Variable(Identifier {
                                name: "msg".to_string()
                            })),
                            delegator: Principal {
                                ident: Identifier {
                                    name: "admin".to_string()
                                }
                            },
                            right: Right::Read,
                            delegated: Principal {
                                ident: Identifier {
                                    name: "alice".to_string()
                                }
                            }
                        }),
                        Box::new(Command::Return(Expr::Value(Value::String(
                            "success".to_string()
                        ))))
                    ))
                ))
            )
        }
    );

    Ok(())
}
