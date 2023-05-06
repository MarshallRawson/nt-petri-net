use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use crate::Token;

pub trait TransitionInputTokens {
    fn from_map(in_map: &mut HashMap<(String, TypeId), Token>) -> Self;
    fn in_edges() -> HashSet<(String, TypeId)>;
}
