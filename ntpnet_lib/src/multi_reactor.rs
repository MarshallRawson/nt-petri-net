use bimap::BiMap;
use crossbeam_channel::{bounded, unbounded, Receiver, Select, Sender};
use std::any::TypeId;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::mem;
use std::thread;

use plotmux::{plotmux::PlotMux, plotsink::PlotSink};

use crate::transition::{Description, Transition};
use crate::{net::Net, PlotOptions, ReactorOptions, Token};

use std::time::Instant;

#[derive(Debug)]
struct TransitionRuntime {
    t: Box<dyn Transition>,
    description: Description,
    in_edge_to_place: BiMap<String, String>,
    out_edge_to_place: BiMap<String, String>,
}

#[derive(Debug)]
struct StateDelta {
    sub: HashSet<(String, TypeId)>,
    add: HashMap<(String, TypeId), &'static str>,
}
impl StateDelta {
    fn make() -> Self {
        Self {
            sub: HashSet::new(),
            add: HashMap::new(),
        }
    }
    fn pop(&mut self, p_ty: &(String, TypeId)) {
        self.sub.insert(p_ty.clone());
    }
    fn push(&mut self, p_ty: &(String, TypeId), ty_name: &'static str) {
        self.add.insert((p_ty.0.clone(), p_ty.1.clone()), ty_name);
    }
}

enum StateBlockable {
    Tokens((TypeId, Token)),
    Terminate(()),
}

#[derive(Debug)]
struct State {
    places: HashMap<String, HashMap<TypeId, VecDeque<Token>>>,
    input_places_idx: BiMap<String, usize>,
    receivers: Vec<Receiver<StateBlockable>>,
    output_places: HashMap<String, Sender<StateBlockable>>,
    state: HashMap<(String, TypeId), (usize, String)>,
    state_exists: HashSet<(String, TypeId)>,
    state_delta: StateDelta,
    state_delta_notification: Sender<StateDelta>,
}
impl State {
    fn make(
        places: HashMap<String, HashMap<TypeId, VecDeque<Token>>>,
        input_places: HashMap<String, Receiver<StateBlockable>>,
        output_places: HashMap<String, Sender<StateBlockable>>,
        state_delta: Sender<StateDelta>,
        exit_rx: Receiver<StateBlockable>,
    ) -> Self {
        let state = {
            let mut state = HashMap::new();
            for (place_name, ty_v) in places.iter() {
                for (ty, v) in ty_v.iter() {
                    state.insert(
                        (place_name.clone(), ty.clone()),
                        (v.len(), (*v[0]).type_name().into()),
                    );
                }
            }
            state
        };
        let state_exists = state
            .iter()
            .filter_map(|(k, v)| if v.0 > 0 { Some(k.clone()) } else { None })
            .collect::<_>();
        let mut input_places_idx = BiMap::new();
        let mut input_places = input_places
            .into_iter()
            .enumerate()
            .map(|(i, (name, rx))| {
                input_places_idx.insert(name.clone(), i);
                rx
            })
            .collect::<Vec<_>>();
        input_places.push(exit_rx);
        Self {
            places: places,
            input_places_idx: input_places_idx,
            receivers: input_places,
            output_places: output_places,
            state: state,
            state_exists: state_exists,
            state_delta: StateDelta::make(),
            state_delta_notification: state_delta,
        }
    }
    fn block_rx(&mut self) -> bool {
        let mut rxs = vec![];
        mem::swap(&mut self.receivers, &mut rxs);
        let mut sel = Select::new();
        for rs in rxs.as_slice() {
            sel.recv(rs);
        }
        let index = sel.ready();
        let exit = if let Ok(send_thing) = rxs[index].recv() {
            match send_thing {
                StateBlockable::Tokens((ty, token)) => {
                    let p_name = self.input_places_idx.get_by_right(&index).unwrap().clone();
                    self.push_local(&(p_name, ty), token);
                    false
                }
                StateBlockable::Terminate(_) => true,
            }
        } else {
            true
        };
        mem::swap(&mut self.receivers, &mut rxs);
        exit
    }
    fn try_rx(&mut self) -> bool {
        let mut rxs = vec![];
        mem::swap(&mut self.receivers, &mut rxs);
        let mut sel = Select::new();
        for rs in rxs.as_slice() {
            sel.recv(rs);
        }
        let mut exit = false;
        while let Ok(index) = sel.try_ready() {
            exit = if let Ok(send_thing) = rxs[index].recv() {
                match send_thing {
                    StateBlockable::Tokens((ty, token)) => {
                        let p_name = self.input_places_idx.get_by_right(&index).unwrap().clone();
                        self.push_local(&(p_name, ty), token);
                        false
                    }
                    StateBlockable::Terminate(_) => true,
                }
            } else {
                break;
            };
        }
        mem::swap(&mut self.receivers, &mut rxs);
        exit
    }
    fn binary(&mut self, plot: Option<(&mut PlotSink, f64)>) -> (bool, &HashSet<(String, TypeId)>) {
        let exit = self.try_rx();
        if let Some((plot, time)) = plot {
            for ((place, _ty), (len, ty_name)) in &self.state {
                plot.plot_series_2d(
                    "local state",
                    &format!("{}/{}", place, ty_name),
                    time,
                    *len as f64,
                );
            }
        }
        (exit, &self.state_exists)
    }
    fn pop(&mut self, p_ty: &(String, TypeId)) -> Token {
        self.state_delta.pop(p_ty);
        *&mut self.state.get_mut(p_ty).unwrap().0 -= 1;
        if self.state[p_ty].0 == 0 {
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
    fn push_local(&mut self, p_ty: &(String, TypeId), t: Token) {
        if !self.places[&p_ty.0].contains_key(&p_ty.1) {
            self.places
                .get_mut(&p_ty.0)
                .unwrap()
                .insert(p_ty.1.clone(), VecDeque::new());
            self.state
                .insert(p_ty.clone(), (0, (*t).type_name().to_string()));
        }
        self.places
            .get_mut(&p_ty.0)
            .unwrap()
            .get_mut(&p_ty.1)
            .unwrap()
            .push_back(t);
        *&mut self.state.get_mut(p_ty).unwrap().0 += 1;
        if !self.state_exists.contains(p_ty) {
            self.state_exists.insert(p_ty.clone());
        }
    }
    fn push(&mut self, p_ty: &(String, TypeId), t: Token) {
        self.state_delta.push(p_ty, (*t).type_name());
        if let Some(out_place) = self.output_places.get_mut(&p_ty.0) {
            out_place
                .send(StateBlockable::Tokens((p_ty.1.clone(), t)))
                .unwrap();
        } else {
            self.push_local(p_ty, t);
        }
    }
    fn state_delta_complete(&mut self) {
        let mut temp = StateDelta::make();
        mem::swap(&mut temp, &mut self.state_delta);
        self.state_delta_notification.send(temp).unwrap();
        self.state_delta = StateDelta::make();
    }
}

#[derive(Debug)]
struct WorkCluster {
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
    fn nonblocking_states(&self) -> HashSet<BTreeSet<(String, TypeId)>> {
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
                            let state_plotting = if plot_options.state {
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
        self.state.places
    }
}

pub struct MultiReactor {
    work_clusters: Vec<Box<dyn FnOnce(Receiver<StateBlockable>) -> WorkCluster + Send>>,
    dots: Vec<(String, String)>,
    pseudo_hashes: Vec<u64>,
    start_state: HashMap<(String, TypeId), (i64, &'static str)>,
    state_delta_monitor: Receiver<StateDelta>,
    monitor_plot: PlotSink,
    reactor_plot: PlotSink,
}

use crate::net::graphviz;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;

impl MultiReactor {
    pub fn png(&self) -> PathBuf {
        assert!(
            self.work_clusters.len() == self.dots.len()
                && self.dots.len() == self.pseudo_hashes.len()
        );
        let hash = {
            let mut s = DefaultHasher::new();
            self.pseudo_hashes.hash(&mut s);
            s.finish()
        };
        let mut dot: String = "digraph NTPnet {\n".into();
        for i in 0..self.work_clusters.len() {
            dot += &format!("subgraph cluster_{} {{\n", i);
            dot += &self.dots[i].0;
            dot += "}\n";
        }
        for i in 0..self.work_clusters.len() {
            dot += &self.dots[i].1;
        }
        dot += "}";
        graphviz(&dot, hash)
    }
    pub fn make(mut net: Net, work_clusters: Vec<HashSet<String>>, plotmux: &mut PlotMux) -> Self {
        let place_io_clusters: HashMap<String, (HashSet<usize>, usize)> = {
            let mut place_io_clusters: HashMap<String, (HashSet<usize>, HashSet<usize>)> = net
                .places
                .iter()
                .map(|(p_name, _)| (p_name.clone(), (HashSet::new(), HashSet::new())))
                .collect::<_>();
            for (t_name, places) in &net.transition_to_places {
                let cluster_idx = work_clusters
                    .iter()
                    .position(|ts| ts.contains(t_name))
                    .unwrap();
                for place in places {
                    if let Some(io_clusters) = place_io_clusters.get_mut(place) {
                        io_clusters.0.insert(cluster_idx);
                    } else {
                        panic!("place {:?} not in {:?}", place, place_io_clusters)
                    }
                }
            }
            for (p_name, transitions) in &net.place_to_transitions {
                for t_name in transitions {
                    let cluster_idx = work_clusters
                        .iter()
                        .position(|ts| ts.contains(t_name))
                        .unwrap();
                    place_io_clusters
                        .get_mut(p_name)
                        .unwrap()
                        .1
                        .insert(cluster_idx);
                }
            }
            for (place, (in_clusters, out_clusters)) in place_io_clusters.iter_mut() {
                if in_clusters.len() != 0 && out_clusters.len() == 0 {
                    out_clusters.insert(in_clusters.iter().next().unwrap().clone());
                } else if in_clusters.len() == 0 && out_clusters.len() != 0 {
                    in_clusters.insert(out_clusters.iter().next().unwrap().clone());
                }
                assert!(
                    out_clusters.len() != 0 && in_clusters.len() != 0,
                    "{} has 0 output clusters: {:#?} and 0 input clusters: {:#?}",
                    place,
                    out_clusters,
                    in_clusters,
                );
            }
            place_io_clusters
                .into_iter()
                .map(|(p_name, (in_c, out_c))| {
                    (p_name, (in_c, out_c.into_iter().reduce(|_, x| x).unwrap()))
                })
                .collect()
        };
        let mut contained_places: HashMap<usize, HashSet<String>> = HashMap::new();
        let mut middle_places = place_io_clusters
            .into_iter()
            .filter_map(|(p_name, (in_c, out_c))| {
                if in_c == HashSet::from([out_c]) {
                    if let Some(work_cluster_places) = contained_places.get_mut(&out_c) {
                        work_cluster_places.insert(p_name);
                    } else {
                        contained_places.insert(out_c, HashSet::from([p_name]));
                    }
                    None
                } else {
                    let in_c = in_c.iter().collect::<Vec<_>>();
                    let (sender, receiver) = unbounded();
                    let mut senders: HashMap<usize, Sender<_>> = in_c[..in_c.len() - 1]
                        .iter()
                        .map(|idx| (**idx, sender.clone()))
                        .collect::<_>();
                    senders.insert(**in_c.last().unwrap(), sender);
                    Some((p_name.clone(), (senders, Some((out_c, receiver)))))
                }
            })
            .collect::<HashMap<String, (HashMap<usize, Sender<_>>, Option<(usize, Receiver<_>)>)>>(
            );
        let mut dots = vec![];
        let mut pseudo_hashes = vec![];
        let (state_delta_notifier, state_delta_monitor) = unbounded();
        let start_state = net
            .start_state()
            .into_iter()
            .map(|((p, ty), (s, n))| ((p, ty), (s as i64, n)))
            .collect();
        Self {
            work_clusters: work_clusters
                .iter()
                .enumerate()
                .map(|(i, cluster)| {
                    let output_places = middle_places
                        .iter()
                        .filter_map(|(p_name, (output_places, _))| {
                            if output_places.contains_key(&i) {
                                Some(p_name.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    let input_places = middle_places
                        .iter()
                        .filter_map(|(p_name, (_, input_places))| {
                            if let Some((cluster, _)) = input_places {
                                if *cluster == i {
                                    return Some(p_name.clone());
                                }
                            }
                            return None;
                        })
                        .collect();
                    let contained_places = contained_places
                        .iter()
                        .filter_map(|(k, v)| if *k == i { Some(v) } else { None })
                        .fold(HashSet::new(), |acc, x| acc.union(x).cloned().collect());
                    let net_split =
                        net.split(&cluster, &input_places, &output_places, &contained_places);
                    let input_places = input_places
                        .iter()
                        .map(|p| {
                            assert_eq!(middle_places[p].1.as_ref().unwrap().0, i);
                            let mut a = None;
                            mem::swap(&mut middle_places.get_mut(p).unwrap().1, &mut a);
                            (p.clone(), a.unwrap().1)
                        })
                        .collect();
                    let output_places = output_places
                        .iter()
                        .map(|p| {
                            (
                                p.clone(),
                                middle_places.get_mut(p).unwrap().0.remove(&i).unwrap(),
                            )
                        })
                        .collect();
                    dots.push(net_split.as_dot(true));
                    pseudo_hashes.push(net_split.pseudo_hash());
                    let plotsink =
                        plotmux.add_plot_sink(&format!("reactor/work_cluster/{:?}", cluster));
                    let sdn = state_delta_notifier.clone();
                    let f: Box<dyn FnOnce(Receiver<StateBlockable>) -> WorkCluster + Send> =
                        Box::new(move |exit_rx| {
                            WorkCluster::make(
                                net_split,
                                input_places,
                                output_places,
                                plotsink,
                                sdn,
                                exit_rx,
                            )
                        });
                    f
                })
                .collect(),
            dots: dots,
            pseudo_hashes: pseudo_hashes,
            start_state: start_state,
            state_delta_monitor: state_delta_monitor,
            monitor_plot: plotmux.add_plot_sink("reactor/monitor"),
            reactor_plot: plotmux.add_plot_sink("reactor"),
        }
    }
    pub fn run(
        mut self,
        plot_options: &Option<ReactorOptions>,
    ) -> HashMap<String, HashMap<TypeId, VecDeque<Token>>> {
        let plot_options: PlotOptions = plot_options.into();
        let mut threads = vec![];
        let mut exit_txs = vec![];
        let (nonblocking_sender, nonblocking_receiver) = bounded(self.work_clusters.len());
        for (i, wc) in self.work_clusters.into_iter().enumerate() {
            let (exit_tx, exit_rx) = bounded(1);
            exit_txs.push(exit_tx);
            let po = plot_options.clone();
            let nbs = nonblocking_sender.clone();
            threads.push(
                thread::Builder::new()
                    .name(format!("work-cluster-{}", i))
                    .spawn(move || {
                        let wc = wc(exit_rx);
                        nbs.send(wc.nonblocking_states()).unwrap();
                        wc.run(po)
                    })
                    .expect(&format!("unable to spawn work-cluster-{} thread", i)),
            );
        }
        let nonblocking_states = {
            (0..threads.len())
                .map(|_| nonblocking_receiver.recv().unwrap())
                .into_iter()
                .flatten()
                .collect::<HashSet<BTreeSet<_>>>()
        };
        let monitor_thread = thread::Builder::new()
            .name("monitor".into())
            .spawn(move || {
                for ((place, _ty), (len, ty_name)) in &self.start_state {
                    if plot_options.monitor {
                        self.monitor_plot.plot_series_2d(
                            "pseudo-state",
                            &format!("{}/{}", place, ty_name),
                            0.0,
                            *len as f64,
                        );
                    }
                }
                let mut state = self.start_state;
                let mut state_binary: BTreeSet<(String, TypeId)> = state.keys().cloned().collect();
                let start = Instant::now();
                loop {
                    let mut deadlock = true;
                    for nonblocking_state in &nonblocking_states {
                        if nonblocking_state.is_subset(&state_binary) {
                            deadlock = false;
                            break;
                        }
                    }
                    if deadlock {
                        break;
                    }
                    if let Ok(state_delta) = self.state_delta_monitor.recv() {
                        let now = (Instant::now() - start).as_secs_f64();
                        for s in state_delta.sub {
                            *&mut state.get_mut(&s).unwrap().0 -= 1;
                            if !state_delta.add.contains_key(&s) && plot_options.monitor {
                                self.monitor_plot.plot_series_2d(
                                    "pseudo-state",
                                    &format!("{}/{}", &s.0, &state[&s].1),
                                    now,
                                    state[&s].0 as f64,
                                );
                            }
                            if state[&s].0 == 0 {
                                state_binary.remove(&s);
                            }
                        }
                        for ((place, ty), ty_name) in state_delta.add {
                            let key = (place, ty);
                            if !state.contains_key(&key) {
                                state.insert(key.clone(), (1, ty_name));
                                state_binary.insert(key.clone());
                            } else {
                                *&mut state.get_mut(&key).unwrap().0 += 1;
                            }
                            if plot_options.monitor {
                                self.monitor_plot.plot_series_2d(
                                    "pseudo-state",
                                    &format!("{}/{}", &key.0, &ty_name),
                                    now,
                                    state[&key].0 as f64,
                                );
                            }
                            if state[&key].0 >= 1 {
                                state_binary.insert(key);
                            }
                        }
                    } else {
                        break;
                    }
                }
                for (i, tx) in exit_txs.into_iter().enumerate() {
                    match tx.send(StateBlockable::Terminate(())) {
                        Ok(_) => {}
                        Err(_) => {
                            self.monitor_plot
                                .println(&format!("failed to terminate work-cluster-{}", i));
                        }
                    }
                }
                if plot_options.monitor {
                    self.monitor_plot.println(&format!(
                        "exiting with state: {:?}",
                        state
                            .iter()
                            .filter(|((_, _), (s, _))| *s != 0)
                            .collect::<Vec<_>>()
                    ));
                }
            })
            .expect("unable to spawn monitor thread");
        let end_state = threads
            .into_iter()
            .enumerate()
            .fold(HashMap::new(), |mut acc, (i, t)| {
                match t.join() {
                    Ok(state) => {
                        for (k, v) in state.into_iter().filter(|(_place, vecs)| {
                            for (_ty, vec) in vecs {
                                if vec.len() > 0 {
                                    return true;
                                }
                            }
                            false
                        }) {
                            acc.insert(k, v);
                        }
                    }
                    Err(_) => {
                        self.reactor_plot
                            .println(&format!("failed to join work-cluster-{}", i));
                    }
                }
                acc
            });
        monitor_thread.join().unwrap();
        end_state
    }
}
