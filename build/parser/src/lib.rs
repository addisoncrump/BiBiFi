//! The bibifi-parser module defines parsing mechanisms and types required for successfully
//! interpreting user input. Additionally, it performs some static analysis to ensure that programs
//! are correct before execution (i.e. naming, delegation, etc).

#![warn(missing_docs)]

/// The members of the AST which will be returned in the parsing result.
pub mod types;
use arrayref::array_ref;
use blake2::{Blake2s, Digest};
use types::*;
use zeroize::Zeroize;

fn hash(input: String) -> [u8; 32] {
    let mut hasher = Blake2s::new();
    hasher.input(input);
    let res = hasher.result();
    *array_ref!(res.as_slice(), 0, 32)
}

peg::parser! {
    grammar program_parser() for str {
        pub rule program() -> Program
            = (comment() "\n")* _ "as" __ "principal" __ p:principal() __ "password" __ s:string() __ "do" _ "\n" cmd:line() _ "***" ("\n" comment())* {
                Program {
                    principal: p,
                    password: hash(s),
                    command: cmd,
                    // principals: HashSet::new(),
                    // variables: HashMap::new(),
                }
            }

        rule principal() -> Principal
            = s:identifier() { Principal { ident: s } }

        rule string() -> String
            = "\"" s:$(['A'..='Z'
                      | 'a'..='z'
                      | '0'..='9'
                      | '_'
                      | ' '
                      | ','
                      | ';'
                      | '\\'
                      | '.'
                      | '?'
                      | '!'
                      | '-' ]*<0,65535>) "\"" { s.to_string() }

        rule line() -> Command
            = c:command() { c }
            / comment() "\n" l:line() { l }

        rule command() -> Command
            = _ "exit" _ (comment() _)? "\n" { Command::Exit }
            / _ "return" __ e:expr() _ (comment() _)? "\n" { Command::Return(e) }
            / _ p:primitive_command() _ (comment() _)? "\n" c:line() { Command::Chain(p, Box::new(c)) }

        rule identifier() -> Identifier
            = s:$(['A'..='Z'
                 | 'a'..='z' ]
                 [ 'A'..='Z'
                 | 'a'..='z'
                 | '0'..='9'
                 | '_']*<0,254>) { Identifier { name: s.to_string() } }

        rule expr() -> Expr
            = v:value() { Expr::Value(v) }
            / "[" _ "]" { Expr::EmptyList }
            / "{" a:( _ a:assignment() _ {a}) ** "," _ "}" { Expr::FieldVals(a) }

        rule primitive_command() -> PrimitiveCommand
            = c:create_principal() { PrimitiveCommand::CreatePrincipal(c) }
            / c:change_password() { PrimitiveCommand::ChangePassword(c) }
            / "set" __ a:assignment() { PrimitiveCommand::Assignment(a) }
            / c:append() { PrimitiveCommand::Append(c) }
            / "local" __ a:assignment() { PrimitiveCommand::LocalAssignment(a) }
            / c:for_each() { PrimitiveCommand::ForEach(c) }
            / "set" __ d:delegation() { PrimitiveCommand::SetDelegation(d) }
            / "delete" __ d:delegation() { PrimitiveCommand::DeleteDelegation(d) }
            / "default" __ "delegator" _ "=" _ p:principal() { PrimitiveCommand::DefaultDelegator(p) }

        rule create_principal() -> CreatePrincipal
            = "create" __ "principal" __ p:principal() __ s:string()
            { CreatePrincipal { principal: p, password: hash(s) } }

        rule change_password() -> ChangePassword
            = "change" __ "password" __ p:principal() __ s:string()
            { ChangePassword { principal: p, password: hash(s) } }

        rule assignment() -> Assignment
            = v:variable() _ "=" _ e:expr()
                { Assignment { variable: v, expr: e } }

        rule append() -> Append
            = "append" __ "to" __ v:variable() __ "with" __ e:expr()
                { Append { variable: v, expr: e } }

        rule for_each() -> ForEach
            = "foreach" __ y:variable() __ "in" __ x:variable() __ "replacewith" __ e:expr()
                { ForEach { value: y, list: x, expr: e } }

        rule delegation() -> Delegation
            = "delegation" __ t:target() __ q:principal() __ r:right() _ "->" _ p:principal()
                {
                    Delegation {
                        target: t,
                        delegator: q,
                        right: r,
                        delegated: p,
                    }
                }

        rule value() -> Value
            = i:identifier() _ "." _ f:variable() { Value::Variable(Variable::Member(i, Box::new(f))) }
            / v:variable() { Value::Variable(v) }
            / s:string() { Value::String(s) }

        rule target() -> Target
            = "all" { Target::All }
            / v:variable() { Target::Variable(v) }

        rule right() -> Right
            = "read" { Right::Read }
            / "write" { Right::Write }
            / "append" { Right::Append }
            / "delegate" { Right::Delegate }

        rule variable() -> Variable
            = i:identifier() { Variable::Variable(i) }

        rule _() = quiet!{ " "* }
        rule __() = quiet!{ " "+ }
        rule comment() = quiet!{ "//"
                 [ 'A'..='Z'
                 | 'a'..='z'
                 | '0'..='9'
                 | '_'
                 | ' '
                 | ','
                 | ';'
                 | '\\'
                 | '.'
                 | '?'
                 | '!'
                 | '-']*}
    }
}

// fn enrich(program: &mut Program) -> Result<(), Box<dyn std::error::Error>> {
//     fn safe_insert(
//         variables: &mut HashMap<Identifier, Scope>,
//         ident: &mut Identifier,
//         scope: Scope,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         if variables.insert(ident.clone(), scope.clone()) == Some(!scope) {
//             return Err(Box::new(std::io::Error::from(
//                 std::io::ErrorKind::InvalidData,
//             )));
//         }
//         Ok(())
//     }
//
//     let mut command = &mut program.command;
//     while let Command::Chain(prim, next) = command {
//         // TODO I need to lock member variables in exprs
//         command = &mut *next;
//         match prim {
//             PrimitiveCommand::CreatePrincipal(cp) => {
//                 program.principals.insert(cp.principal.ident.clone());
//             }
//             PrimitiveCommand::ChangePassword(cp) => {
//                 program.principals.insert(cp.principal.ident.clone());
//             }
//             PrimitiveCommand::Assignment(a) => {
//                 if let Variable::Variable(ident) = &mut a.variable {
//                     safe_insert(&mut program.variables, ident, Scope::Global)?;
//                 } else {
//                     panic!("Encountered a member variable in an assignment statement.");
//                 }
//             }
//             PrimitiveCommand::Append(a) => {
//                 if let Variable::Variable(ident) = &mut a.variable {
//                     if !program.variables.contains_key(ident) {
//                         // assert it's global
//                         safe_insert(&mut program.variables, ident, Scope::Global)?;
//                     }
//                 } else {
//                     panic!("Encountered a member variable in an append statement.");
//                 }
//             }
//             PrimitiveCommand::LocalAssignment(a) => {
//                 if let Variable::Variable(ident) = &mut a.variable {
//                     safe_insert(&mut program.variables, ident, Scope::Local)?;
//                 } else {
//                     panic!("Encountered a member variable in an assignment statement.");
//                 }
//             }
//             PrimitiveCommand::ForEach(fe) => {
//                 if let Variable::Variable(ident) = &mut fe.value {
//                     if program.variables.contains_key(ident) {
//                         return Err(Box::new(std::io::Error::from(
//                             std::io::ErrorKind::InvalidData,
//                         )));
//                     }
//                 } else {
//                     panic!("Encountered a member variable as a iter value in a foreach statement.");
//                 }
//                 if let Variable::Variable(ident) = &mut fe.list {
//                     safe_insert(&mut program.variables, ident, Scope::Global)?;
//                 } else {
//                     panic!("Encountered a member variable as a list in a foreach statement.");
//                 }
//             }
//             PrimitiveCommand::SetDelegation(_) => {}
//             PrimitiveCommand::DeleteDelegation(_) => {}
//             PrimitiveCommand::DefaultDelegator(_) => {}
//         }
//     }
//
//     Ok(())
// }

#[derive(Zeroize)]
#[zeroize(drop)]
struct ZeroisingString(String);

/// Main entrypoint for the parser. Provide a program as a string, you get a program returned. Easy!
pub fn parse(program: String) -> Result<Program, Box<dyn std::error::Error>> {
    let wrap = ZeroisingString(program);
    if wrap.0.len() > 1000000 || !wrap.0.is_ascii() {
        Err(Box::new(std::io::Error::from(
            std::io::ErrorKind::InvalidData,
        )))
    } else {
        match program_parser::program(&wrap.0) {
            Ok(program) => {
                // enrich(&mut program)?;
                Ok(program)
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

#[cfg(test)]
mod tests;
