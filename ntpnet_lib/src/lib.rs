use std::any::Any;

pub type Token = Box<dyn Any>;

pub mod fire;
pub mod product;
pub mod transition;




