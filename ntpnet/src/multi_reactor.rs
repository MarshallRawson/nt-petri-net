use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use std::any::TypeId;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::mem;
use std::thread;

use plotmux::{plotmux::PlotMux, plotsink::PlotSink};

use crate::{
    monitor::monitor_thread,
    net::Net,
    state::{StateBlockable, StateDelta},
    work_cluster::WorkCluster,
    PlotOptions, ReactorOptions, Token,
};

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
        let monitor_thread = monitor_thread(
            self.start_state,
            (0..threads.len())
                .map(|_| nonblocking_receiver.recv().unwrap())
                .into_iter()
                .flatten()
                .collect::<HashSet<BTreeSet<_>>>(),
            self.state_delta_monitor,
            exit_txs,
            self.monitor_plot,
            plot_options,
        );
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
        drop(monitor_thread);
        end_state
    }
}
