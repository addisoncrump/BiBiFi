use std::collections::{HashMap, HashSet};
use std::ops::Not;

#[derive(Clone, PartialEq, Debug)]
pub struct Program {
    pub principal: Principal,
    pub password: String,
    pub command: Command,
    pub principals: HashSet<Identifier>,
    pub variables: HashMap<Identifier, Scope>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Principal {
    pub ident: Identifier,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    Exit,
    Return(Expr),
    Chain(PrimitiveCommand, Box<Command>),
}

#[derive(Clone, PartialEq, Debug)]
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

#[derive(Clone, PartialEq, Debug)]
pub struct CreatePrincipal {
    pub principal: Principal,
    pub password: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ChangePassword {
    pub principal: Principal,
    pub password: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Assignment {
    pub variable: Variable,
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Append {
    pub variable: Variable,
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ForEach {
    pub value: Variable,
    pub list: Variable,
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Delegation {
    pub target: Target,
    pub delegator: Principal,
    pub right: Right,
    pub delegated: Principal,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Expr {
    Value(Value),
    EmptyList,
    FieldVals(Vec<Assignment>),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Variable(Variable),
    String(String),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Variable {
    Variable(Identifier),
    Member(Box<Variable>, Box<Variable>), // nested values possible, but not implemented
}

#[derive(Clone, PartialEq, Debug)]
pub enum Target {
    All,
    Variable(Variable),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Right {
    Read,
    Write,
    Append,
    Delegate,
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum Scope {
    Local,
    Global,
}

impl Not for Scope {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Scope::Local => Scope::Global,
            Scope::Global => Scope::Local,
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Debug)]
pub struct Identifier {
    pub name: String,
}
