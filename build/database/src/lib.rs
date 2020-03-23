use serde::Serialize;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Database {
    principals: HashMap<String, Box<VPrincipal>>,
    variables: HashMap<String, Value>,
    default: Box<VPrincipal>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum VPrincipal {
    Admin([u8; 32]),
    Anyone(Principal),
    User(Principal, [u8; 32]),
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Principal {
    delegations: Vec<Delegation>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub enum Value {
    Immediate(String),
    List(Vec<Box<Value>>),
    FieldVals(HashMap<String, String>),
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Delegation {
    target: Target,
    delegator: Box<VPrincipal>,
    right: Right,
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Target {
    All,
    Variable(String),
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Right {
    Read,
    Write,
    Append,
    Delegate,
}

impl Database {
    pub fn new(admin_hash: [u8; 32]) -> Database {
        let mut principals: HashMap<String, Box<VPrincipal>> = HashMap::new();
        principals.insert("admin".to_string(), Box::new(VPrincipal::Admin(admin_hash)));
        let anyone = Box::new(VPrincipal::Anyone(Principal {
            delegations: Vec::new(),
        }));
        principals.insert("anyone".to_string(), anyone.clone());
        Database {
            principals,
            variables: HashMap::new(),
            default: anyone,
        }
    }

    pub fn check_pass(&self, principal: &String, hash: &[u8; 32]) -> bool {
        self.principals
            .get(principal)
            .map(|principal| match **principal {
                VPrincipal::Anyone(_) => false,
                VPrincipal::User(_, checked) | VPrincipal::Admin(checked) => &checked == hash,
            })
            .unwrap_or(false)
    }

    /// Preconditions: delegator and delegated must exist, you should check if acting principal has right
    pub fn delegate(
        &mut self,
        target: &Target,
        delegator: &String,
        right: &Right,
        delegated: &String,
    ) {
        let delegator = self
            .principals
            .get(delegator)
            .expect("Precondition of delegator existence not met.");
        let delegated = self
            .principals
            .get(delegated)
            .cloned()
            .expect("Precondition of delegated existence not met.");
        match *delegated {
            VPrincipal::Admin(_) => {}
            VPrincipal::Anyone(mut p) | VPrincipal::User(mut p, _) => {
                let delegation = Delegation {
                    target: target.clone(),
                    delegator: delegator.clone(),
                    right: right.clone(),
                };
                p.delegations.push(delegation);
            }
        }
    }

    /// Preconditions: delegator and delegated must exist, you should check if acting principal has right
    pub fn undelegate(
        &mut self,
        target: &Target,
        delegator: &String,
        right: &Right,
        delegated: &String,
    ) {
        let delegator = self
            .principals
            .get(delegator)
            .expect("Precondition of delegator existence not met.");
        let delegated = self
            .principals
            .get(delegated)
            .cloned()
            .expect("Precondition of delegated existence not met.");
        match *delegated {
            VPrincipal::Admin(_) => {}
            VPrincipal::Anyone(mut p) | VPrincipal::User(mut p, _) => {
                let delegation = Delegation {
                    target: target.clone(),
                    delegator: delegator.clone(),
                    right: right.clone(),
                };
                p.delegations.retain(|d| d != &delegation);
            }
        }
    }

    /// Preconditions: none
    pub fn check_principal(&self, principal: &String) -> bool {
        self.principals.contains_key(principal)
    }

    /// Preconditions: none, but you should check if principal exists first
    pub fn create_principal(&mut self, principal: &String, hash: &[u8; 32]) {
        self.principals.insert(
            principal.clone(),
            Box::new(VPrincipal::User(
                Principal {
                    delegations: Vec::new(),
                },
                hash.clone(),
            )),
        );
    }

    /// Preconditions: principal must exist, and you should check if current user is admin or principal
    pub fn change_password(&self, principal: &String, hash: &[u8; 32]) {
        let mut principal = self
            .principals
            .get(principal)
            .cloned()
            .expect("Precondition of principal existence not met.");
        match *principal {
            VPrincipal::Anyone(_) => {}
            VPrincipal::User(_, ref mut existing) | VPrincipal::Admin(ref mut existing) => {
                *existing = hash.clone()
            }
        }
    }

    /// Preconditions: principal must exist
    pub fn check_right(&self, target: &Target, right: &Right, principal: &String) -> bool {
        let principal = self
            .principals
            .get(principal)
            .cloned()
            .expect("Precondition of principal existence not met.");
        match *principal {
            VPrincipal::Admin(_) => true,
            VPrincipal::Anyone(p) | VPrincipal::User(p, _) => {
                let mut searched: Vec<&Delegation> = Vec::new();
                let mut searching: VecDeque<&Delegation> = p
                    .delegations
                    .iter()
                    .filter(|d| &d.target == target && &d.right == right)
                    .collect();
                while !searching.is_empty() {
                    let curr = searching.pop_front().unwrap(); // guaranteed by while conditionx
                    if searched.contains(&curr) {
                        continue;
                    }
                    searched.push(curr);
                    match &*curr.delegator {
                        VPrincipal::Admin(_) => return true,
                        VPrincipal::Anyone(p) | VPrincipal::User(p, _) => p
                            .delegations
                            .iter()
                            .filter(|d| &d.target == target && &d.right == right)
                            .for_each(|d| searching.push_back(d)),
                    }
                }
                false
            }
        }
    }

    /// Preconditions: none, but you should probably check if the user has rights
    pub fn set(&mut self, variable: &String, value: &Value) {
        self.variables.insert(variable.clone(), value.clone());
    }

    /// Preconditions: non, but you should probably check if the user has rights
    pub fn get(&mut self, variable: &String) -> Option<&Value> {
        self.variables.get(variable)
    }

    /// Preconditions: non, but you should probably check if the user has rights
    pub fn get_mut(&mut self, variable: &String) -> Option<&mut Value> {
        self.variables.get_mut(variable)
    }
}

#[cfg(test)]
mod tests;
