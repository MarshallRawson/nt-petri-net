use std::collections::{HashSet, HashMap};
use std::any::TypeId;

use crate::Token;

pub struct TransitionCase {
    input_conditions: Vec<HashSet<(String, TypeId)>>,
    callback: String,
    products: Vec<HashSet<(String, TypeId)>>,
}
pub trait Transition {
    fn in_edges(&self) -> HashSet<(String, TypeId)>;
    fn out_edges(&self) -> HashSet<(String, TypeId)>;
    fn transitions(&self) -> Vec<TransitionCase>;
    fn call(&self, map: &mut HashMap<String, Token>) -> u64;
}





