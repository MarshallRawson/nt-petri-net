use std::collections::{HashSet, HashMap};
use std::any::TypeId;

use crate::Token;

pub struct FireToProduct {
    input_condition: HashSet<(String, TypeId)>,
    callback: String,
    products: HashSet<(String, TypeId)>,
}
pub trait Transition {
    fn in_edges() -> HashSet<String>;
    fn out_edges() -> HashSet<String>;
    fn fire_to_product() -> Vec<FireToProduct>;
    fn call(map: &mut HashMap<String, Token>) -> String;
}





