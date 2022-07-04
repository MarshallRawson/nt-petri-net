use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::marker::Send;

pub mod graph;

pub struct PlaceMaker {
    pub make: Box<dyn FnOnce() -> Box<dyn Place> + Send>,
    pub in_type: Box<dyn Fn() -> TypeId + Send>,
    pub out_types: Box<dyn Fn() -> HashSet<TypeId> + Send>,
    pub out_types_names: Box<dyn Fn() -> HashSet<String> + Send>,
}
#[macro_export]
macro_rules! PlaceMaker {
    ($expression:expr) => {
        PlaceMaker {
            make: $expression,
            in_type: Box::new(Self::in_type),
            out_types: Box::new(Self::out_types),
            out_types_names: Box::new(Self::out_types_names),
        }
    };
}

pub trait Place {
    fn run(&mut self, x: Box<dyn Any + Send>, out_map: &mut HashMap<TypeId, Edge>) -> TypeId;
}

#[derive(Debug)]
pub struct Edge {
    _name: String,
    type_name: String,
    type_id: TypeId,
    vec: VecDeque<Box<dyn Any + Send>>,
}
impl Edge {
    pub fn push(&mut self, x: Box<dyn Any + Send>) {
        assert_eq!((&*x).type_id(), self.type_id);
        self.vec.push_back(x);
    }
    pub fn pop(&mut self) -> Box<dyn Any + Send> {
        self.vec.pop_front().unwrap()
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
}
unsafe impl Send for Edge {}
