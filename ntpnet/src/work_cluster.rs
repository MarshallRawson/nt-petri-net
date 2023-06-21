use bimap::BiMap;
use crossbeam_channel::{Receiver, Sender};
use std::any::TypeId;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use plotmux::plotsink::PlotSink;

use crate::transition::{Description, Transition};
use crate::{
    net::Net,
    state::{State, StateBlockable, StateDelta},
    PlotOptions, Token,
};

use std::time::Instant;

#[derive(Debug)]
struct TransitionRuntime {
    t: Box<dyn Transition>,
    description: Description,
    in_edge_to_place: BiMap<String, String>,
    out_edge_to_place: BiMap<String, String>,
}

#[derive(Debug)]
pub struct WorkCluster {
    transitions: HashMap<String, TransitionRuntime>,
    state: State,
    plot_sink: PlotSink,
}
impl WorkCluster {
    pub fn make(
        n: Net,
        input_places: HashMap<String, Receiver<StateBlockable>>,
        output_places: HashMap<String, Sender<StateBlockable>>,
        plot_sink: PlotSink,
        state_delta_notification: Sender<StateDelta>,
        exit_rx: Receiver<StateBlockable>,
    ) -> Self {
        let transitions = n
            .transitions
            .into_iter()
            .map(|(name, t_maker)| {
                let t = t_maker();
                let mut d = t.description();
                let in_edge_to_place = n
                    .pt_edges
                    .iter()
                    .filter(|((_, t), _)| t == &name)
                    .map(|((p, _), e)| (e.clone(), p.clone()))
                    .collect::<BiMap<String, String>>();
                let out_edge_to_place = n
                    .tp_edges
                    .iter()
                    .filter(|((t, _), _)| t == &name)
                    .map(|((_, p), e)| (e.clone(), p.clone()))
                    .collect::<BiMap<String, String>>();
                for (_, case) in d.cases.iter_mut() {
                    for condition in case.inputs.iter_mut() {
                        *condition = condition
                            .iter()
                            .map(|(edge, ty)| {
                                (
                                    in_edge_to_place
                                        .get_by_left(edge)
                                        .expect(&format!(
                                            "{}: {} not found on left of {:#?}",
                                            name, edge, in_edge_to_place
                                        ))
                                        .clone(),
                                    ty.clone(),
                                )
                            })
                            .collect::<HashSet<_>>();
                    }
                    for product in case.outputs.iter_mut() {
                        *product = product
                            .iter()
                            .map(|(edge, ty)| {
                                (
                                    out_edge_to_place
                                        .get_by_left(edge)
                                        .expect(&format!(
                                            "{}: {} not found on left of {:#?}",
                                            name, edge, out_edge_to_place
                                        ))
                                        .clone(),
                                    ty.clone(),
                                )
                            })
                            .collect::<_>();
                    }
                }
                (
                    name,
                    TransitionRuntime {
                        t: t,
                        description: d,
                        in_edge_to_place: in_edge_to_place,
                        out_edge_to_place: out_edge_to_place,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        Self {
            state: State::make(
                n.places,
                input_places,
                output_places,
                state_delta_notification,
                exit_rx,
            ),
            transitions: transitions,
            plot_sink: plot_sink,
        }
    }
    pub fn nonblocking_states(&self) -> HashSet<BTreeSet<(String, TypeId)>> {
        self.transitions
            .iter()
            .map(|(_, t_run)| &t_run.description.cases)
            .flatten()
            .map(|(_, case)| {
                case.inputs
                    .iter()
                    .map(|cond| cond.iter().cloned().collect())
                    .collect::<HashSet<_>>()
            })
            .flatten()
            .collect()
    }
    pub fn run(
        mut self,
        plot_options: PlotOptions,
    ) -> HashMap<String, HashMap<TypeId, VecDeque<Token>>> {
        let start = Instant::now();
        if plot_options.reactor_timing {
            self.plot_sink
                .plot_series_2d("reactor timing", "blocking", 0.0, 0.0);
            self.plot_sink
                .plot_series_2d("reactor timing", "nonblocking", 0.0, 0.0);
        }
        if plot_options.transition_timing {
            for (t_name, _t_run) in &self.transitions {
                self.plot_sink
                    .plot_series_2d("transition timing", t_name, 0.0, 0.0);
            }
        }
        let mut exit = false;
        while !exit {
            let mut blocked = false;
            while !blocked {
                let mut last_nonblocking_time = (Instant::now() - start).as_secs_f64();
                blocked = true;
                for (t_name, t_run) in self.transitions.iter_mut() {
                    for (f_name, case) in &t_run.description.cases {
                        for (i, condition) in case.inputs.iter().enumerate() {
                            let state_plotting = if plot_options.local_state {
                                Some((&mut self.plot_sink, (Instant::now() - start).as_secs_f64()))
                            } else {
                                None
                            };
                            let e_bin = self.state.binary(state_plotting);
                            exit = e_bin.0;
                            if (condition - e_bin.1).len() == 0 {
                                let mut in_map = HashMap::new();
                                for p_ty in condition {
                                    in_map.insert(
                                        (
                                            t_run
                                                .in_edge_to_place
                                                .get_by_right(&p_ty.0)
                                                .unwrap()
                                                .clone(),
                                            p_ty.1.clone(),
                                        ),
                                        self.state.pop(p_ty),
                                    );
                                }
                                let mut out_map = HashMap::new();
                                let elapsed = (Instant::now() - start).as_secs_f64();
                                if plot_options.reactor_timing {
                                    self.plot_sink.plot_series_2d(
                                        "reactor timing",
                                        "nonblocking",
                                        elapsed,
                                        elapsed - last_nonblocking_time,
                                    );
                                }
                                t_run.t.call(&f_name, i, &mut in_map, &mut out_map);
                                let elapsed2 = (Instant::now() - start).as_secs_f64();
                                last_nonblocking_time = elapsed2;
                                if plot_options.transition_timing {
                                    self.plot_sink.plot_series_2d(
                                        "transition timing",
                                        &t_name,
                                        elapsed2,
                                        elapsed2 - elapsed,
                                    );
                                }
                                for ((e_name, ty), t) in out_map.into_iter() {
                                    let place = t_run
                                        .out_edge_to_place
                                        .get_by_left(&e_name)
                                        .unwrap()
                                        .clone();
                                    self.state.push(&(place, ty), t);
                                }
                                self.state.state_delta_complete();
                                blocked = false;
                                break;
                            }
                        }
                    }
                }
            }
            if !exit {
                let elapsed = (Instant::now() - start).as_secs_f64();
                exit = self.state.block_rx();
                if plot_options.reactor_timing {
                    let elapsed2 = (Instant::now() - start).as_secs_f64();
                    let blocking_time = elapsed2 - elapsed;
                    self.plot_sink.plot_series_2d(
                        "reactor timing",
                        "blocking",
                        elapsed2,
                        blocking_time,
                    );
                }
            }
        }
        self.state.take_places()
    }
}
