use std::collections::{HashSet, HashMap};
use std::any::TypeId;

use crate::Token;

pub struct TransitionDescr {
    pub in_edges: HashSet<(String, TypeId)>,
    pub out_edges: HashSet<(String, TypeId)>,
    pub cases: HashMap<String, TransitionCase>,
}
pub struct TransitionCase {
    pub conditions: Vec<HashSet<(String, TypeId)>>,
    pub products: Vec<HashSet<(String, TypeId)>>,
}

pub trait Transition {
    fn descr(&self) -> TransitionDescr;
    fn call(&mut self, case: &str, condition: usize,
            in_map: &mut HashMap<(String, TypeId), Token>,
            out_map: &mut HashMap<(String, TypeId), Token>,
    ) -> usize;
}
