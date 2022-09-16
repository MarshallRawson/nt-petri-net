use std::collections::{HashMap, HashSet};
use std::any::TypeId;

use crate::Token;

pub trait Product {
    fn into_map(self: Self, map: &mut HashMap::<(String, TypeId), Token>);
    fn out_edges() -> HashSet<(String, TypeId)>;
}
