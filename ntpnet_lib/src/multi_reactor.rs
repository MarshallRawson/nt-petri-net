use bimap::BiMap;
use std::any::TypeId;
use std::mem;
use std::thread;
use std::collections::{HashMap, HashSet, VecDeque};
use crossbeam_channel::{unbounded, Receiver, Select, Sender};

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
    input_places_idx: BiMap<String, usize>,
    receivers: Vec<Receiver<(TypeId, Token)>>,
    output_places: HashMap<String, Sender<(TypeId, Token)>>,
    state: HashMap<(String, TypeId), usize>,
    state_exists: HashSet<(String, TypeId)>,
}
impl State {
    fn make(mut places: HashMap<String, HashMap<TypeId, VecDeque<Token>>>, input_places: HashMap<String, Receiver<(TypeId, Token)>>, output_places: HashMap<String, Sender<(TypeId, Token)>>) -> Self {
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
        let mut input_places_idx = BiMap::new();
        let input_places = input_places.into_iter().enumerate().map(|(i, (name, rx))| {
            input_places_idx.insert(name.clone(), i);
            rx
        }).collect();
        Self {
            places: places,
            input_places_idx: input_places_idx,
            receivers: input_places,
            output_places: output_places,
            state: state,
            state_exists: state_exists,
        }
    }
    fn block_rx(&mut self) {
        self.try_rx();
        let mut rxs = vec![];
        mem::swap(&mut self.receivers, &mut rxs);
        let mut sel = Select::new();
        for rs in rxs.as_slice() {
            sel.recv(rs);
        }
        let index = sel.ready();
        if let Ok((ty, token)) = rxs[index].recv() {
            let p_name = self.input_places_idx.get_by_right(&index).unwrap().clone();
            self.push(&(p_name, ty), token);
        }
        mem::swap(&mut self.receivers, &mut rxs);
    }
    fn try_rx(&mut self) {
        let mut rxs = vec![];
        mem::swap(&mut self.receivers, &mut rxs);
        let mut sel = Select::new();
        for rs in rxs.as_slice() {
            sel.recv(rs);
        }
        while let Ok(index) = sel.try_ready() {
            if let Ok((ty, token)) = rxs[index].try_recv() {
                let p_name = self.input_places_idx.get_by_right(&index).unwrap().clone();
                self.push(&(p_name, ty), token);
            }
        }
        mem::swap(&mut self.receivers, &mut rxs);
    }
    fn binary(&mut self) -> &HashSet<(String, TypeId)> {
        self.try_rx();
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
        if let Some(out_place) = self.output_places.get_mut(&p_ty.0) {
            out_place.send((p_ty.1.clone(), t)).unwrap();
        } else {
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
}

#[derive(Debug)]
struct WorkCluster {
    transitions: HashMap<String, TransitionRuntime>,
    state: State,
    plot_sink: PlotSink,
}
impl WorkCluster {
    pub fn make(n: Net, input_places: HashMap<String, Receiver<(TypeId, Token)>>, output_places: HashMap<String, Sender<(TypeId, Token)>>, plot_sink: PlotSink) -> Self {
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
                                    in_edge_to_place.get_by_left(edge).unwrap().clone(),
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
                    }
                )
            })
            .collect::<HashMap<_, _>>();
        Self {
            state: State::make(n.places, input_places, output_places),
            transitions: transitions,
            plot_sink: plot_sink,
        }
    }
    pub fn run(mut self) {
        let start = Instant::now();
        self.plot_sink.plot_series_2d("reactor timing", "blocking", 0.0, 0.0);
        self.plot_sink.plot_series_2d("reactor timing", "nonblocking", 0.0, 0.0);
        for (t_name, _t_run) in &self.transitions {
            self.plot_sink.plot_series_2d("reactor timing", t_name, 0.0, 0.0);
        }
        loop {
            let mut blocked = false;
            while !blocked {
                let mut last_nonblocking_time = (Instant::now() - start).as_secs_f64();
                blocked = true;
                for (t_name, t_run) in self.transitions.iter_mut() {
                    for (f_name, case) in &t_run.description.cases {
                        for (i, condition) in case.inputs.iter().enumerate() {
                            if (condition - self.state.binary()).len() == 0 {
                                self.plot_sink.println(&format!("{:?}", self.state.binary()));
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
                                let nonblocking_time = elapsed - last_nonblocking_time;
                                self.plot_sink.plot_series_2d("reactor timing", "nonblocking", elapsed, nonblocking_time);
                                t_run.t.call(&f_name, i, &mut in_map, &mut out_map);
                                let elapsed2 = (Instant::now() - start).as_secs_f64();
                                last_nonblocking_time = elapsed2;
                                let t_time = elapsed2 - elapsed;
                                self.plot_sink.plot_series_2d("reactor timing", &t_name, elapsed2, t_time);
                                for ((e_name, ty), t) in out_map.into_iter() {
                                    let place = t_run
                                        .out_edge_to_place
                                        .get_by_left(&e_name)
                                        .unwrap()
                                        .clone();
                                    self.state.push(&(place, ty), t);
                                }
                                blocked = false;
                                break;
                            }
                        }
                    }
                    if !blocked {
                        t_run.description.cases.rotate_left(1);
                    }
                }
            }
            let elapsed = (Instant::now() - start).as_secs_f64();
            self.state.block_rx();
            let elapsed2 = (Instant::now() - start).as_secs_f64();
            let blocking_time = elapsed2 - elapsed;
            self.plot_sink.plot_series_2d("reactor timing", "blocking", elapsed2, blocking_time);
        }
    }
}

pub struct MultiReactor {
    work_clusters: Vec<Box::<dyn FnOnce() -> WorkCluster + Send>>,
    dots: Vec<(String, String)>,
    pseudo_hashes: Vec<u64>,
}

use crate::net::graphviz;
use std::path::PathBuf;
use std::hash::Hash;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

impl MultiReactor {
    pub fn png(&self) -> PathBuf {
        assert!(self.work_clusters.len() == self.dots.len() && self.dots.len() == self.pseudo_hashes.len());
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
        let place_io_clusters : HashMap<String, (HashSet<usize>, usize)> = {
            let mut place_io_clusters : HashMap<String, (HashSet<usize>, HashSet<usize>)> = net.places.iter().map(|(p_name, _)|
                (p_name.clone(), (HashSet::new(), HashSet::new()))
            ).collect::<_>();
            for (t_name, places) in &net.transition_to_places {
                let cluster_idx = work_clusters.iter().position(|ts| ts.contains(t_name)).unwrap();
                for place in places {
                    place_io_clusters.get_mut(place).unwrap().0.insert(cluster_idx);
                }
            }
            for (p_name, transitions) in &net.place_to_transitions {
                for t_name in transitions {
                    let cluster_idx = work_clusters.iter().position(|ts| ts.contains(t_name)).unwrap();
                    place_io_clusters.get_mut(p_name).unwrap().1.insert(cluster_idx);
                }
            }
            for (place, (_, out_clusters)) in &place_io_clusters {
                assert!(out_clusters.len() == 1, "{} has not 1 output clusters: {:#?}", place, out_clusters);
            }
            place_io_clusters.into_iter().map(|(p_name, (in_c, out_c))|
                (p_name, (in_c, out_c.into_iter().reduce(|_, x| x).unwrap()))
            ).collect()
        };
        let mut contained_places : HashMap<usize, HashSet<String>> = HashMap::new();
        let mut middle_places = place_io_clusters.into_iter().filter_map(|(p_name, (in_c, out_c))| {
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
                let mut senders : HashMap<usize, Sender<(TypeId, Token)>> = in_c[..in_c.len()-1].iter().map(|idx|
                    (**idx, sender.clone())).collect::<_>()
                ;
                senders.insert(**in_c.last().unwrap(), sender);
                Some((p_name.clone(), (senders, Some((out_c, receiver)))))
            }
        }).collect::<HashMap<String, (HashMap<usize, Sender<(TypeId, Token)>>, Option<(usize, Receiver<(TypeId, Token)>)>)>>();
        let mut dots = vec![];
        let mut pseudo_hashes = vec![];
        Self {
            work_clusters: work_clusters.iter().enumerate().map(|(i, cluster)| {
                let output_places = middle_places.iter().filter_map(|(p_name, (output_places, _))| {
                    if output_places.contains_key(&i) { Some(p_name.clone()) }
                    else { None }
                }).collect();
                let input_places = middle_places.iter().filter_map(|(p_name, (_, input_places))| {
                    if let Some((cluster, _)) = input_places {
                        if *cluster == i {
                            return Some(p_name.clone());
                        }
                    }
                    return None;
                }).collect();
                let contained_places = contained_places
                    .iter()
                    .filter_map(|(k, v)| if *k == i { Some(v) } else { None })
                    .fold(HashSet::new(), |acc, x| acc.union(x).cloned().collect());
                let net_split = net.split(&cluster, &input_places, &output_places, &contained_places);
                let input_places = input_places.iter().map(|p| {
                    assert_eq!(middle_places[p].1.as_ref().unwrap().0, i);
                    let mut a = None;
                    mem::swap(&mut middle_places.get_mut(p).unwrap().1, &mut a);
                    (p.clone(), a.unwrap().1)
                }).collect();
                let output_places = output_places.iter().map(|p| (p.clone(), middle_places.get_mut(p).unwrap()
                    .0.remove(&i).unwrap())).collect();
                dots.push(net_split.as_dot(true));
                pseudo_hashes.push(net_split.pseudo_hash());
                let plotsink = plotmux.add_plot_sink(&format!("{:?}", cluster));
                let f: Box::<dyn FnOnce() -> WorkCluster + Send> = Box::new(move || {
                    WorkCluster::make(net_split, input_places, output_places, plotsink)
                });
                f
            }).collect(),
            dots: dots,
            pseudo_hashes: pseudo_hashes,
        }
    }
    pub fn run(self) {
        let mut threads = vec![];
        for wc in self.work_clusters.into_iter() {
            threads.push(thread::spawn(move || wc().run()));
        }
        for t in threads {
            t.join().unwrap();
        }
    }
}
