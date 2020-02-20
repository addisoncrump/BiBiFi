#[derive(Clone)]
pub struct Program {
    pub principal: Principal,
    pub password: String,
    pub command: Command,
}

#[derive(Clone)]
pub struct Principal {
    pub ident: Identifier,
}

#[derive(Clone)]
pub enum Command {
    Exit,
    Return(Expr),
    Chain(PrimitiveCommand, Box<Command>),
}

#[derive(Clone)]
pub enum PrimitiveCommand {
    CreatePrincipal(CreatePrincipal),
    ChangePassword(ChangePassword),
    Assignment(Assignment),
    Append(Append),
    LocalAssignment(Assignment),
    ForEach(ForEach),
    SetDelegation(Delegation),
    DeleteDelegation(Delegation),
    DefaultDelegator(Principal),
}

#[derive(Clone)]
pub struct CreatePrincipal {
    pub principal: Principal,
    pub password: String,
}

#[derive(Clone)]
pub struct ChangePassword {
    pub principal: Principal,
    pub password: String,
}

#[derive(Clone)]
pub struct Assignment {
    pub variable: Variable,
    pub expr: Expr,
}

#[derive(Clone)]
pub struct Append {
    pub variable: Variable,
    pub expr: Expr,
}

#[derive(Clone)]
pub struct ForEach {
    pub value: Variable,
    pub list: Variable,
    pub expr: Expr,
}

#[derive(Clone)]
pub struct Delegation {
    pub target: Target,
    pub delegator: Principal,
    pub right: Right,
    pub delegated: Principal,
}

#[derive(Clone)]
pub enum Expr {
    Value(Value),
    EmptyList,
    FieldVals(Vec<Assignment>),
}

#[derive(Clone)]
pub enum Value {
    Variable(Variable),
    String(String),
}

#[derive(Clone)]
pub enum Variable {
    Variable(Identifier),
    Member(Box<Variable>, Box<Variable>), // nested values possible, but not implemented
}

#[derive(Clone)]
pub enum Target {
    All,
    Variable(Variable),
}

#[derive(Clone)]
pub enum Right {
    Read,
    Write,
    Append,
    Delegate,
}

#[derive(Clone)]
pub struct Identifier {
    pub name: String,
}

peg::parser! {
    grammar program_parser() for str {
        pub rule program() -> Program
            = _ "as" __ "principal" __ p:principal() __ "password" __ s:string() __ "do" _ "\n" cmd:command() _ "***" {
                Program {
                    principal: p,
                    password: s,
                    command: cmd
                }
            }

        rule principal() -> Principal
            = s:identifier() { Principal { ident: s } }

        rule string() -> String
            = "\"" s:$(['A'..='Z'
                      | 'a'..='z'
                      | '0'..='9'
                      | ','
                      | ';'
                      | '\\'
                      | '.'
                      | '?'
                      | '!'
                      | '-' ]*) "\"" { s.to_string() }

        rule command() -> Command
            = _ "exit" _ "\n" { Command::Exit }
            / _ "return" __ e:expr() _ "\n" { Command::Return(e) }
            / _ p:primitive_command() _ "\n" c:command() { Command::Chain(p, Box::new(c)) }

        rule identifier() -> Identifier
            = s:$(['A'..='Z'
                 | 'a'..='z' ]
                 [ 'A'..='Z'
                 | 'a'..='z'
                 | '0'..='9'
                 | '_']*) { Identifier { name: s.to_string() } }

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
                { CreatePrincipal { principal: p, password: s } }

        rule change_password() -> ChangePassword
            = "change" __ "password" __ p:principal() __ s:string()
                { ChangePassword { principal: p, password: s } }

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
            = v:variable() { Value::Variable(v) }
            / v:variable() _ "." _ f:variable() { Value::Variable(Variable::Member(Box::new(v), Box::new(f))) }
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
    }
}

pub use program_parser::program as parse;
