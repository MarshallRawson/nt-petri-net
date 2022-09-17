use std::collections::{HashSet, HashMap};
use std::any::TypeId;

use crate::Token;

#[derive(Debug)]
pub struct Description {
    pub in_edges: HashSet<(String, TypeId)>,
    pub out_edges: HashSet<(String, TypeId)>,
    pub cases: HashMap<String, Case>,
}
#[derive(Debug)]
pub struct Case {
    pub conditions: Vec<HashSet<(String, TypeId)>>,
    pub products: Vec<HashSet<(String, TypeId)>>,
}

pub trait Transition {
    fn description(&self) -> Description;
    fn call(&mut self, case: &str, condition: usize,
            in_map: &mut HashMap<(String, TypeId), Token>,
            out_map: &mut HashMap<(String, TypeId), Token>,
    ) -> usize;
}

use std::fmt::{Debug, Result};
impl Debug for dyn Transition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result {
        write!(f, "Transition{{{:#?}}}", self.description())
    }
}

