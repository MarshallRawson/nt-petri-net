use std::collections::{HashSet, HashMap};
use std::any::TypeId;

use crate::Token;

pub struct Description {
    pub in_edges: HashSet<(String, TypeId)>,
    pub out_edges: HashSet<(String, TypeId)>,
    pub cases: HashMap<String, Case>,
}
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
