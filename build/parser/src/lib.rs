//! The bibifi-parser module defines parsing mechanisms and types required for successfully
//! interpreting user input. Additionally, it performs some static analysis to ensure that programs
//! are correct before execution (i.e. naming, delegation, etc).

/// The members of the AST which will be returned in the parsing result.
pub mod types;
use bibifi_util::hash;
use types::*;

peg::parser! {
    grammar program_parser() for str {
        pub rule program<'a>() -> Program
            = (comment() "\n")* _ "as" __ "principal" __ !keyword() p:principal() __ "password" __ s:string() __ "do" _ comment()? "\n"
                    (comment() "\n")*
                    cmd:(a:line() "\n" { a })*
                    term:terminator_command() "\n"
                    _ "***" _ (comment() _) ** "\n" {
                Program {
                    principal: p,
                    password: hash(s),
                    commands: cmd,
                    terminator: term
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

        rule line() -> PrimitiveCommand
            = _ c:primitive_command() _ (comment() _) ** "\n" { c }

        rule terminator_command() -> TerminatorCommand
            = _ "exit" _ (comment() _) ** "\n" { TerminatorCommand::Exit }
            / _ "return" __ e:expr() _ (comment() _) ** "\n" { TerminatorCommand::Return(e) }

        rule identifier() -> Identifier
            = s:$(['A'..='Z'
                 | 'a'..='z' ]
                 [ 'A'..='Z'
                 | 'a'..='z'
                 | '0'..='9'
                 | '_']*<0,254>) { Identifier { name: s.to_string() } }

        rule expr() -> Expr
            = !keyword() v:value() { Expr::Value(v) }
            / "[" _ "]" { Expr::EmptyList }
            / "{" a:( _ () a:root_value_assignment() _ {a}) ** "," _ "}" { Expr::FieldVals(a) }

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

        rule root_value_assignment() -> Assignment
            = !keyword() i:identifier() _ "=" _ v:value()
                { Assignment { variable: Variable::Variable(i), expr: Expr::Value(v) }}

        rule root_assignment() -> Assignment
            = !keyword() i:identifier() _ "=" _ e:expr()
                { Assignment { variable: Variable::Variable(i), expr: e } }

        rule member_assignment() -> Assignment
            = !keyword() i1:identifier() "." !keyword() i2:identifier() _ "=" _ e:expr()
                { Assignment { variable: Variable::Member(i1, Box::new(Variable::Variable(i2))), expr: e } }

        rule append() -> Append
            = "append" __ "to" __ !keyword() i:identifier() __ "with" __ e:expr()
                { Append { variable: Variable::Variable(i), expr: e } }

        rule for_each() -> ForEach
            = "foreach" __ y:variable() __ "in" __ x:variable() __ "replacewith" __ e:expr()
                { ForEach { value: y, list: x, expr: e } }

        rule delegation() -> Delegation
            = "delegation" __ t:target() __ !keyword() q:principal() __ r:right() _ "->" _ !keyword() p:principal()
                {
                    Delegation {
                        target: t,
                        delegator: q,
                        right: r,
                        delegated: p,
                    }
                }

        rule value() -> Value
            = !keyword() i:identifier() _ "." _ !keyword() f:identifier() { Value::Variable(Variable::Member(i, Box::new(Variable::Variable(f)))) }
            / v:variable() { Value::Variable(v) }
            / s:string() { Value::String(s) }

        rule target() -> Target
            = "all" { Target::All }
            / !keyword() i:identifier() { Target::Variable(i) }

        rule right() -> Right
            = "read" { Right::Read }
            / "write" { Right::Write }
            / "append" { Right::Append }
            / "delegate" { Right::Delegate }

        rule variable() -> Variable
            = !keyword() i:identifier() { Variable::Variable(i) }

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

        rule keyword() = quiet!{
            "all" / "append" / "as" / "change" / "create" / "default" / "delegation" / "delegator"
                  / "delete" / "do" / "exit" / "foreach" / "in" / "local" / "password" / "principal"
                  / "read" / "replacewith" / "return" / "set" / "to" / "write" / "***"
        }
    }
}

/// Main entrypoint for the parser. Provide a program as a string, you get a program returned. Easy!
pub fn parse(program: String) -> Result<Program, Box<dyn std::error::Error>> {
    if program.len() > 1000000 || !program.is_ascii() {
        Err(Box::new(std::io::Error::from(
            std::io::ErrorKind::InvalidData,
        )))
    } else {
        match program_parser::program(&program) {
            Ok(program) => Ok(program),
            Err(e) => Err(Box::new(e)),
        }
    }
}

#[cfg(test)]
mod tests;
