use std::collections::{HashMap, VecDeque};

use crate::{Token, TransitionMaker, transition::Transition, net::Net};

struct WorkCluster(Net);
impl WorkCluster {
    pub fn run(self) {
        let transitions = self.0.transitions
            .into_iter()
            .map(|(name, t_maker)| (name, t_maker()))
            .collect::<HashMap<_, _>>()
        ;
    }
}

pub struct Reactor {
    work_cluster: WorkCluster,
}

impl Reactor {
    pub fn make(net: Net) -> Self {
        Self {
            work_cluster: WorkCluster(net),
        }
    }
    pub fn run(self) {
        self.work_cluster.run();
    }
}




