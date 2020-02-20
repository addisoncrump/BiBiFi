use super::*;
use std::error::Error;

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
