//! The bibifi-parser module defines parsing mechanisms and types required for successfully
//! interpreting user input. Additionally, it performs some static analysis to ensure that programs
//! are correct before execution (i.e. naming, delegation, etc).

#![warn(missing_docs)]

/// The members of the AST which will be returned in the parsing result.
pub mod types;
use crate::types::Scope::{Global, Local};
use arrayref::array_ref;
use blake2::{Blake2s, Digest};
use std::collections::{HashMap, HashSet};
use std::error::Error;
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
        pub rule program<'a>() -> Program
            = (comment() "\n")* _ "as" __ "principal" __ p:principal() __ "password" __ s:string() __ "do" _ "\n" cmd:line() _ "***" ("\n" comment())* {
                Program {
                    principal: p,
                    password: hash(s),
                    command: cmd,
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
            / "local" __ a:root_assignment() { PrimitiveCommand::LocalAssignment(a) }
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
            = a:root_assignment() { a }
            / a:member_assignment() { a }

        rule root_assignment() -> Assignment
            = i:identifier() _ "=" _ e:expr()
                { Assignment { variable: Variable::Variable(i), expr: e } }

        rule member_assignment() -> Assignment
            = i1:identifier() "." i2:identifier() _ "=" _ e:expr()
                { Assignment { variable: Variable::Member(i1, Box::new(Variable::Variable(i2))), expr: e } }

        rule append() -> Append
            = "append" __ "to" __ i:identifier() __ "with" __ e:expr()
                { Append { variable: Variable::Variable(i), expr: e } }

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
            / i:identifier() { Target::Variable(i) }

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
            Ok(mut program) => Ok(program),
            Err(e) => Err(Box::new(e)),
        }
    }
}

#[cfg(test)]
mod tests;
