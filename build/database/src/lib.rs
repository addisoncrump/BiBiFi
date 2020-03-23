use serde::Serialize;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Database {
    principals: HashMap<String, VPrincipal>,
    variables: HashMap<String, Value>,
    default: String,
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
    delegator: String,
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
        let mut principals = HashMap::new();
        principals.insert("admin".to_string(), VPrincipal::Admin(admin_hash));
        let anyone = VPrincipal::Anyone(Principal {
            delegations: Vec::new(),
        });
        principals.insert("anyone".to_string(), anyone.clone());
        Database {
            principals,
            variables: HashMap::new(),
            default: "anyone".to_string(),
        }
    }

    pub fn check_pass(&self, principal: &String, hash: &[u8; 32]) -> bool {
        self.principals
            .get(principal)
            .map(|principal| match *principal {
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
        assert!(self.principals.contains_key(delegator));
        let pdelegated = self
            .principals
            .get_mut(delegated)
            .expect("Precondition of delegated existence not met.");
        match pdelegated {
            VPrincipal::Admin(_) => {}
            VPrincipal::Anyone(ref mut p) | VPrincipal::User(ref mut p, _) => {
                let delegation = Delegation {
                    target: target.clone(),
                    delegator: delegator.clone(),
                    right: right.clone(),
                };
                p.delegations.push(delegation);
            }
        };
    }

    /// Preconditions: delegator and delegated must exist, you should check if acting principal has right
    pub fn undelegate(
        &mut self,
        target: &Target,
        delegator: &String,
        right: &Right,
        delegated: &String,
    ) {
        assert!(self.principals.contains_key(delegator));
        let delegated = self
            .principals
            .get_mut(delegated)
            .expect("Precondition of delegated existence not met.");
        match delegated {
            VPrincipal::Admin(_) => {}
            VPrincipal::Anyone(ref mut p) | VPrincipal::User(ref mut p, _) => {
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
            VPrincipal::User(
                Principal {
                    delegations: Vec::new(),
                },
                hash.clone(),
            ),
        );
    }

    /// Preconditions: principal must exist, and you should check if current user is admin or principal
    pub fn change_password(&mut self, principal: &String, hash: &[u8; 32]) {
        let mut principal = self
            .principals
            .get_mut(principal)
            .expect("Precondition of principal existence not met.");
        match principal {
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
            .expect("Precondition of principal existence not met.");
        match principal {
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
                    match self.principals.get(&curr.delegator).unwrap() {
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
