use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet, VecDeque};

pub mod graph;

pub trait Place {
    fn in_type(&self) -> TypeId;
    fn out_types(&self) -> HashSet<TypeId>;
    fn out_types_names(&self) -> HashSet<String>;
    fn run(&mut self, x: Box<dyn Any>, out_map: &mut HashMap::<TypeId, Edge>) -> TypeId;
}

#[derive(Debug)]
pub struct Edge {
    _name: String,
    type_name: String,
    type_id : TypeId,
    vec : VecDeque<Box<dyn Any>>,
}
impl Edge {
    pub fn push(&mut self, x: Box<dyn Any>) {
        assert_eq!((&*x).type_id(), self.type_id);
        self.vec.push_back(x);
    }
    pub fn pop(&mut self) -> Box<dyn Any> {
        self.vec.pop_front().unwrap()
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
}
