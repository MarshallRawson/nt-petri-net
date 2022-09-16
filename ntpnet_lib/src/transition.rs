use std::collections::{HashSet, HashMap};
use std::any::TypeId;

use crate::Token;

pub struct TransitionDescr {
    in_edges: HashSet<(String, TypeId)>,
    out_edges: HashSet<(String, TypeId)>,
    cases: HashMap<String, TransitionCase>,
}
pub struct TransitionCase {
    conditions: Vec<HashSet<(String, TypeId)>>,
    products: Vec<HashSet<(String, TypeId)>>,
}

pub trait Transition {
    fn descr(&self) -> TransitionDescr;
    fn call(&mut self, case: &String, condition: usize,
            in_map: &mut HashMap<(String, TypeId), Token>)
        -> (usize, HashMap<(String, TypeId), Token>);
}





