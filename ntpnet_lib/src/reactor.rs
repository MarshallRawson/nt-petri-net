use bimap::BiMap;
use std::any::TypeId;
use std::collections::{HashMap, HashSet, VecDeque};

use plotmux::{plotmux::PlotMux, plotsink::PlotSink};

use crate::transition::{Description, Transition};
use crate::{net::Net, Token};

use std::time::Instant;

#[derive(Debug)]
struct TransitionRuntime {
    t: Box<dyn Transition>,
    description: Description,
    in_edge_to_place: BiMap<String, String>,
    out_edge_to_place: BiMap<String, String>,
}

#[derive(Debug)]
struct State {
    places: HashMap<String, HashMap<TypeId, VecDeque<Token>>>,
    state: HashMap<(String, TypeId), usize>,
    state_exists: HashSet<(String, TypeId)>,
}
impl State {
    fn make(mut places: HashMap<String, HashMap<TypeId, VecDeque<Token>>>) -> Self {
        let state = {
            let mut state = HashMap::new();
            for (place_name, ty_v) in places.iter_mut() {
                for (ty, v) in ty_v.iter() {
                    state.insert((place_name.clone(), ty.clone()), v.len());
                }
            }
            state
        };
        let state_exists = state
            .iter()
            .filter_map(|(k, v)| if *v > 0 { Some(k.clone()) } else { None })
            .collect::<_>();
        Self {
            places: places,
            state: state,
            state_exists: state_exists,
        }
    }
    fn binary(&self) -> &HashSet<(String, TypeId)> {
        &self.state_exists
    }
    fn pop(&mut self, p_ty: &(String, TypeId)) -> Token {
        *self.state.get_mut(p_ty).unwrap() -= 1;
        if *self.state.get_mut(p_ty).unwrap() == 0 {
            self.state_exists.remove(p_ty);
        }
        self.places
            .get_mut(&p_ty.0)
            .unwrap()
            .get_mut(&p_ty.1)
            .unwrap()
            .pop_front()
            .unwrap()
    }
    fn push(&mut self, p_ty: &(String, TypeId), t: Token) {
        if !self.places[&p_ty.0].contains_key(&p_ty.1) {
            self.places
                .get_mut(&p_ty.0)
                .unwrap()
                .insert(p_ty.1.clone(), VecDeque::new());
            self.state.insert(p_ty.clone(), 0);
        }
        self.places
            .get_mut(&p_ty.0)
            .unwrap()
            .get_mut(&p_ty.1)
            .unwrap()
            .push_back(t);
        *self.state.get_mut(p_ty).unwrap() += 1;
        if !self.state_exists.contains(p_ty) {
            self.state_exists.insert(p_ty.clone());
        }
    }
}

#[derive(Debug)]
struct WorkCluster {
    transitions: HashMap<String, TransitionRuntime>,
    state: State,
    plot_sink: PlotSink,
}
impl WorkCluster {
    pub fn make(n: Net, plot_sink: PlotSink) -> Self {
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
                                            "{}: in edge {} not found on the left of {:#?}",
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
                                    out_edge_to_place.get_by_left(edge).unwrap().clone(),
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
            state: State::make(n.places),
            transitions: transitions,
            plot_sink: plot_sink,
        }
    }
    pub fn run(mut self) {
        let start = Instant::now();
        self.plot_sink
            .plot_series_2d("reactor timing", "nonblocking", 0.0, 0.0);
        for (t_name, _t_run) in &self.transitions {
            self.plot_sink
                .plot_series_2d("reactor timing", t_name, 0.0, 0.0);
        }
        let mut blocked = false;
        while !blocked {
            let mut last_nonblocking_time = (Instant::now() - start).as_secs_f64();
            blocked = true;
            for (t_name, t_run) in self.transitions.iter_mut() {
                for (f_name, case) in &t_run.description.cases {
                    for (i, condition) in case.inputs.iter().enumerate() {
                        if (condition - self.state.binary()).len() == 0 {
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
                            self.plot_sink.plot_series_2d(
                                "reactor timing",
                                "nonblocking",
                                elapsed,
                                elapsed - last_nonblocking_time,
                            );
                            t_run.t.call(&f_name, i, &mut in_map, &mut out_map);
                            let elapsed2 = (Instant::now() - start).as_secs_f64();
                            last_nonblocking_time = elapsed2;
                            self.plot_sink.plot_series_2d(
                                "reactor timing",
                                t_name,
                                elapsed2,
                                elapsed2 - elapsed,
                            );
                            for ((e_name, ty), t) in out_map.into_iter() {
                                let place = t_run
                                    .out_edge_to_place
                                    .get_by_left(&e_name)
                                    .unwrap()
                                    .clone();
                                self.state.push(&(place, ty), t);
                            }
                            blocked = false;
                        }
                    }
                }
            }
        }
    }
}

pub struct Reactor {
    work_cluster: WorkCluster,
}

impl Reactor {
    pub fn make(net: Net, plotmux: &mut PlotMux) -> Self {
        Self {
            work_cluster: WorkCluster::make(net, plotmux.add_plot_sink("work_cluster0")),
        }
    }
    pub fn run(self) {
        self.work_cluster.run();
        println!("Done!");
    }
}
