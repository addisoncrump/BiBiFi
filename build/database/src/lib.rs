use crate::Status::{DENIED, FAILED, SUCCESS};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Database {
    principals: HashMap<String, VPrincipal>,
    variables: HashMap<String, Value>,
    def_delegator: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum VPrincipal {
    Admin([u8; 32]),
    Anyone(Principal),
    User(Principal, [u8; 32]),
}

impl ToString for VPrincipal {
    fn to_string(&self) -> String {
        match self {
            VPrincipal::Admin(_) => "admin".to_string(),
            VPrincipal::Anyone(p) | VPrincipal::User(p, _) => p.to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Principal {
    name: String,
    delegations: Vec<Delegation>,
}

impl ToString for Principal {
    fn to_string(&self) -> String {
        self.name.clone()
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(untagged)]
pub enum Value {
    Immediate(String),
    List(Vec<Value>),
    FieldVals(HashMap<String, String>),
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Delegation {
    target: String,
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

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum Status {
    SUCCESS,
    DENIED,
    FAILED,
}

impl Database {
    pub fn new(admin_hash: [u8; 32]) -> Database {
        let mut principals = HashMap::new();
        principals.insert("admin".to_string(), VPrincipal::Admin(admin_hash));
        let anyone = VPrincipal::Anyone(Principal {
            name: "anyone".to_string(),
            delegations: Vec::new(),
        });
        principals.insert("anyone".to_string(), anyone.clone());
        Database {
            principals,
            variables: HashMap::new(),
            def_delegator: "anyone".to_string(),
        }
    }

    #[must_use]
    pub fn check_pass(&self, principal: &String, hash: &[u8; 32]) -> Status {
        self.principals
            .get(principal)
            .map(|principal| match *principal {
                VPrincipal::Anyone(_) => DENIED,
                VPrincipal::User(_, checked) | VPrincipal::Admin(checked) => {
                    if &checked == hash {
                        SUCCESS
                    } else {
                        DENIED
                    }
                }
            })
            .unwrap_or(FAILED)
    }

    #[must_use]
    pub fn delegate(
        &mut self,
        user: &String,
        target: &Target,
        delegator: &String,
        right: &Right,
        delegated: &String,
    ) -> Status {
        if user == "admin" || user == delegator {
            if let Some(pdelegator) = self.principals.get(delegator).cloned() {
                if let Some(pdelegated) = self.principals.get(delegated).cloned() {
                    let mut p = match pdelegated {
                        VPrincipal::Admin(_) => {
                            if let Target::Variable(variable) = target {
                                if !(self.variables.contains_key(variable)) {
                                    return FAILED;
                                } else if !(self.direct_check_right(
                                    variable,
                                    &Right::Delegate,
                                    &pdelegator,
                                )) {
                                    return DENIED;
                                }
                            }
                            return SUCCESS;
                        }
                        VPrincipal::Anyone(ref p) | VPrincipal::User(ref p, _) => p.clone(),
                    };
                    if let Target::Variable(variable) = target {
                        if user == "admin"
                            || self.direct_check_right(variable, &Right::Delegate, &pdelegator)
                        {
                            let delegation = Delegation {
                                target: variable.clone(),
                                delegator: delegator.to_string(),
                                right: right.clone(),
                            };
                            p.delegations.push(delegation);
                        } else {
                            return DENIED;
                        }
                    } else {
                        for variable in self.variables.keys() {
                            if user == "admin"
                                || self.direct_check_right(variable, &Right::Delegate, &pdelegator)
                            {
                                let delegation = Delegation {
                                    target: variable.clone(),
                                    delegator: delegator.to_string(),
                                    right: right.clone(),
                                };
                                p.delegations.push(delegation);
                            }
                        }
                    }
                    match pdelegated {
                        VPrincipal::Anyone(_) => self
                            .principals
                            .insert("anyone".to_string(), VPrincipal::Anyone(p)),
                        VPrincipal::User(_, hash) => self
                            .principals
                            .insert(p.name.clone(), VPrincipal::User(p, hash.clone())),
                        _ => panic!(),
                    };
                    return SUCCESS;
                }
            }
        }
        FAILED
    }

    #[must_use]
    pub fn undelegate(
        &mut self,
        user: &String,
        target: &Target,
        delegator: &String,
        right: &Right,
        delegated: &String,
    ) -> Status {
        if user == "admin" || user == delegator || user == delegated {
            if self.principals.contains_key(delegator) {
                if let Some(pdelegated) = self.principals.get(delegated).cloned() {
                    match pdelegated {
                        VPrincipal::Admin(_) => {}
                        VPrincipal::Anyone(ref p) | VPrincipal::User(ref p, _) => {
                            let mut p = p.clone();
                            let delegations = if let Target::Variable(variable) = target {
                                if user == delegated
                                    || self.check_right(variable, &Right::Delegate, user)
                                {
                                    let delegation = Delegation {
                                        target: variable.clone(),
                                        delegator: delegator.clone(),
                                        right: right.clone(),
                                    };
                                    vec![delegation]
                                } else {
                                    return DENIED;
                                }
                            } else {
                                self.variables
                                    .keys()
                                    .clone()
                                    .filter_map(|variable| {
                                        if user == delegated
                                            || self.check_right(variable, &Right::Delegate, user)
                                        {
                                            Some(Delegation {
                                                target: variable.clone(),
                                                delegator: delegator.clone(),
                                                right: right.clone(),
                                            })
                                        } else {
                                            None
                                        }
                                    })
                                    .collect()
                            };
                            p.delegations.retain(|d| !delegations.contains(d));
                            match pdelegated {
                                VPrincipal::Anyone(_) => self
                                    .principals
                                    .insert("anyone".to_string(), VPrincipal::Anyone(p)),
                                VPrincipal::User(_, hash) => self
                                    .principals
                                    .insert(p.name.clone(), VPrincipal::User(p, hash.clone())),
                                _ => panic!(),
                            };
                            return SUCCESS;
                        }
                    }
                }
            }
        }
        FAILED
    }

    #[must_use]
    pub fn set_default_delegator(&mut self, user: &String, delegator: &String) -> Status {
        if user == "admin" {
            self.def_delegator = delegator.clone();
            SUCCESS
        } else {
            DENIED
        }
    }

    #[must_use]
    pub fn create_principal(
        &mut self,
        user: &String,
        principal: &String,
        hash: &[u8; 32],
    ) -> Status {
        if user != "admin" {
            DENIED
        } else if self.principals.contains_key(principal) {
            FAILED
        } else {
            let name = principal;
            let principal = Principal {
                name: name.clone(),
                delegations: Vec::new(),
            };
            if self.principals.contains_key(&self.def_delegator) {
                self.principals.insert(
                    principal.name.clone(),
                    VPrincipal::User(principal, hash.clone()),
                );
                for right in &[Right::Read, Right::Write, Right::Append, Right::Delegate] {
                    match self.delegate(
                        user,
                        &Target::All,
                        &self.def_delegator.clone(),
                        right,
                        name,
                    ) {
                        SUCCESS => {}
                        _ => panic!(),
                    }
                }
                SUCCESS
            } else {
                FAILED
            }
        }
    }

    #[must_use]
    pub fn change_password(
        &mut self,
        user: &String,
        principal: &String,
        hash: &[u8; 32],
    ) -> Status {
        if user == "admin" || user == principal {
            if let Some(principal) = self.principals.get_mut(principal) {
                match principal {
                    VPrincipal::User(_, ref mut existing) | VPrincipal::Admin(ref mut existing) => {
                        *existing = hash.clone();
                        SUCCESS
                    }
                    VPrincipal::Anyone(_) => FAILED,
                }
            } else {
                FAILED
            }
        } else {
            DENIED
        }
    }

    #[must_use]
    fn check_right(&self, target: &String, right: &Right, principal: &String) -> bool {
        let principal = self
            .principals
            .get(principal)
            .expect("Precondition of principal existence not met.");
        self.direct_check_right(target, right, principal)
    }

    #[must_use]
    fn direct_check_right(&self, target: &String, right: &Right, principal: &VPrincipal) -> bool {
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

    #[must_use]
    pub fn set(&mut self, user: &String, variable: &String, value: &Value) -> Status {
        if !self.variables.contains_key(variable) {
            self.variables.insert(variable.clone(), value.clone());
            for right in &[Right::Read, Right::Write, Right::Append, Right::Delegate] {
                match self.delegate(
                    &"admin".to_string(),
                    &Target::All,
                    &"admin".to_string(),
                    right,
                    user,
                ) {
                    SUCCESS => {}
                    _ => panic!(),
                }
            }
            SUCCESS
        } else if self.check_right(variable, &Right::Write, user) {
            self.variables.insert(variable.clone(), value.clone());
            SUCCESS
        } else {
            DENIED
        }
    }

    #[must_use]
    pub fn set_member(
        &mut self,
        user: &String,
        variable: &String,
        member: &String,
        value: &String,
    ) -> Status {
        if let Some(existing) = self.variables.get(variable).cloned() {
            match existing {
                Value::FieldVals(mut fv) => {
                    if let Some(existing) = fv.get_mut(member) {
                        if self.check_right(variable, &Right::Write, user) {
                            *existing = value.clone();
                            self.variables
                                .insert(variable.clone(), Value::FieldVals(fv));
                            SUCCESS
                        } else {
                            DENIED
                        }
                    } else {
                        FAILED
                    }
                }
                _ => FAILED,
            }
        } else {
            FAILED
        }
    }

    #[must_use]
    pub fn append(&mut self, user: &String, variable: &String, value: &Value) -> Status {
        match value {
            Value::Immediate(_) | Value::FieldVals(_) => {
                if let Some(existing) = self.variables.get(variable).cloned() {
                    if self.check_right(variable, &Right::Write, user)
                        || self.check_right(variable, &Right::Append, user)
                    {
                        if let Value::List(mut list) = existing {
                            list.push(value.clone());
                            self.variables.insert(variable.clone(), Value::List(list));
                            SUCCESS
                        } else {
                            FAILED
                        }
                    } else {
                        DENIED
                    }
                } else {
                    FAILED
                }
            }
            Value::List(list) => {
                if let Some(existing) = self.variables.get(variable).cloned() {
                    if self.check_right(variable, &Right::Write, user)
                        || self.check_right(variable, &Right::Append, user)
                    {
                        if let Value::List(mut elist) = existing {
                            for item in list {
                                elist.push(item.clone());
                            }
                            self.variables.insert(variable.clone(), Value::List(elist));
                            SUCCESS
                        } else {
                            FAILED
                        }
                    } else {
                        DENIED
                    }
                } else {
                    FAILED
                }
            }
        }
    }

    #[must_use]
    pub fn get(&self, user: &String, variable: &String) -> Result<&Value, Status> {
        if !self.variables.contains_key(variable) {
            Err(FAILED)
        } else if self.check_right(variable, &Right::Read, user) {
            Ok(self.variables.get(variable).unwrap())
        } else {
            Err(DENIED)
        }
    }

    pub fn contains(&self, variable: &String) -> bool {
        self.variables.contains_key(variable)
    }
}

#[cfg(test)]
#[forbid(unused_must_use)]
mod tests;
