use std::collections::{HashMap, HashSet};
use std::any::TypeId;

use crate::Token;

pub trait Fire {
    fn from_map(map: &mut HashMap<String, Token>) -> Self;
    fn edges() -> HashSet<(String, TypeId)>;
}
