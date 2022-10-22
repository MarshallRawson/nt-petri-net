use std::any::Any;

pub type Token = Box<dyn Any + Send>;

pub mod net;
pub mod reactor;
pub mod multi_reactor;
pub mod transition;
pub mod transition_input_tokens;
pub mod transition_output_tokens;

pub type TransitionMaker = Box<dyn FnOnce() -> Box<dyn transition::Transition> + Send>;
