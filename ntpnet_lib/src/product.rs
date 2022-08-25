use std::collections::{HashMap, HashSet};
use std::any::TypeId;

use crate::Token;

pub trait Product {
    fn into_map(self: Self, map: &mut HashMap::<String, Token>);
    fn edges(&self) -> HashSet<(String, TypeId)>;
}
