#![allow(incomplete_features)]

extern crate ntpnet_macro;
pub use ntpnet_macro::{Transition, TransitionInputTokensMacro, TransitionOutputTokensMacro};

use std::any::Any;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;

pub trait NamedAny: Any {
    fn type_name(&self) -> &'static str;
}
impl<T: Any> NamedAny for T {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

pub struct Token(Box<dyn NamedAny + Send>);
impl Token {
    pub fn new<T: NamedAny + Send>(t: T) -> Self {
        assert!(!<dyn Any>::is::<Self>(&t));
        Self(Box::new(t))
    }
    pub fn downcast<T: 'static>(self) -> Result<Box<T>, Box<(dyn Any)>> {
        <Box<dyn Any>>::downcast::<T>(self.0)
    }
}
impl Debug for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Token").finish_non_exhaustive()
    }
}
impl Deref for Token {
    type Target = dyn NamedAny + Send;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

mod memory_monitor;
mod multi_reactor;
mod pseudo_state_monitor;
pub use multi_reactor::MultiReactor;
mod net;
pub use net::Net;
mod reactor;
pub use reactor::reactor;
mod state;
pub mod transition;
mod transition_input_tokens;
pub use transition_input_tokens::TransitionInputTokens;
mod transition_output_tokens;
pub use transition_output_tokens::TransitionOutputTokens;
mod work_cluster;

pub type TransitionMaker = Box<dyn FnOnce() -> Box<dyn transition::Transition> + Send>;

use clap::{Args, Subcommand};
#[derive(Subcommand)]
pub enum ReactorOptions {
    PlotOptions(PlotOptions),
}
#[derive(Args, Clone, Default)]
pub struct PlotOptions {
    #[arg(short, long)]
    local_state: bool,
    #[arg(short, long)]
    reactor_timing: bool,
    #[arg(short, long)]
    transition_timing: bool,
    #[arg(short, long)]
    pseudo_state: bool,
    #[arg(short, long)]
    memory_profile: Option<f64>,
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
