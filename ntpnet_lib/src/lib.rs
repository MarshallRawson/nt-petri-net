#![feature(trait_upcasting)]
#![allow(incomplete_features)]

use std::any::Any;
use std::fmt;
use std::fmt::{Debug, Formatter};

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

use clap::{Args, Subcommand};
#[derive(Subcommand)]
pub enum ReactorOptions {
    PlotOptions(PlotOptions),
}
#[derive(Args, Clone, Default)]
pub struct PlotOptions {
    #[arg(short, long)]
    state: bool,
    #[arg(short, long)]
    reactor_timing: bool,
    #[arg(short, long)]
    transition_timing: bool,
    #[arg(short, long)]
    monitor: bool,
}

impl From<&Option<ReactorOptions>> for PlotOptions {
    fn from(opts: &Option<ReactorOptions>) -> Self {
        match opts {
            Some(opts) => match opts {
                ReactorOptions::PlotOptions(p) => p.clone(),
            },
            None => PlotOptions::default(),
        }
    }
}
