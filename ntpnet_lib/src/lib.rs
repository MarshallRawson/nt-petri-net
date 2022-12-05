#![feature(trait_upcasting)]
#![allow(incomplete_features)]

use std::fmt;
use std::fmt::{Debug, Formatter};
use std::any::Any;

pub trait NamedAny: Any {
    fn type_name(&self) -> &'static str;
}

impl<T: Any> NamedAny for T {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

pub type Token = Box<dyn NamedAny + Send>;

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        (self as &dyn Any).fmt(f)
    }
}

pub mod multi_reactor;
pub mod net;
pub mod reactor;
pub mod transition;
pub mod transition_input_tokens;
pub mod transition_output_tokens;

pub type TransitionMaker = Box<dyn FnOnce() -> Box<dyn transition::Transition> + Send>;
