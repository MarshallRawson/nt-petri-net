use std::any::Any;

pub type Token = Box<dyn Any>;

pub mod net;
pub mod reactor;
pub mod fire;
pub mod product;
pub mod transition;

pub type TransitionMaker = Box<dyn FnOnce() -> Box<dyn transition::Transition> + Send>;
