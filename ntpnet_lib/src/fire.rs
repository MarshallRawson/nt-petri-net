use std::collections::{HashMap, HashSet};
use std::any::TypeId;

use crate::Token;

pub trait Fire {
    fn from_map(in_map: &mut HashMap<(String, TypeId), Token>) -> Self;
    fn in_edges() -> HashSet<(String, TypeId)>;
}
