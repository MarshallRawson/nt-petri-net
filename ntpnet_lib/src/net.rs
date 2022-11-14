use itertools::Itertools;
use std::any::TypeId;
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque};
use std::env;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;


use crate::{Token, TransitionMaker};
pub struct Net {
    pub transitions: HashMap<String, TransitionMaker>,
    pub places: HashMap<String, HashMap<TypeId, VecDeque<Token>>>,
    pub transition_to_places: HashMap<String, HashSet<String>>,
    pub place_to_transitions: HashMap<String, HashSet<String>>,
    pub pt_edges: HashMap<(String, String), String>,
    pub tp_edges: HashMap<(String, String), String>,
}
impl Net {
    pub fn make() -> Self {
        Self {
            transitions: HashMap::new(),
            places: HashMap::new(),
            transition_to_places: HashMap::new(),
            place_to_transitions: HashMap::new(),
            pt_edges: HashMap::new(),
            tp_edges: HashMap::new(),
        }
    }
    pub fn split(&mut self, transitions: &HashSet<String>, input_places: &HashSet<String>, output_places: &HashSet<String>, contained_places: &HashSet<String>) -> Self {
        let mut right = Self::make();
        for t_name in transitions {
            right.transitions.insert(t_name.clone(), self.transitions.remove(t_name).unwrap());
            right.transition_to_places.insert(t_name.clone(), self.transition_to_places.remove(t_name).unwrap());
            for p_name in &right.transition_to_places[t_name] {
                let id = (t_name.clone(), p_name.clone());
                let edge = self.tp_edges.remove(&id).unwrap();
                right.tp_edges.insert(id, edge);
            }
        }
        for p_name in contained_places {
            right.places.insert(p_name.clone(), self.places.remove(p_name).unwrap());
            right.place_to_transitions.insert(p_name.clone(), self.place_to_transitions.remove(p_name).unwrap());
            for t_name in &right.place_to_transitions[p_name] {
                let id = (p_name.clone(), t_name.clone());
                let edge = self.pt_edges.remove(&id).unwrap();
                right.pt_edges.insert(id, edge);
            }
        }
        for p_name in input_places {
            right.places.insert(p_name.clone(), self.places.remove(p_name).unwrap());
            let intersecting_transitions = self.place_to_transitions[p_name]
                .intersection(transitions)
                .cloned()
                .collect::<HashSet<_>>();
            self.place_to_transitions.insert(p_name.clone(), &self.place_to_transitions[p_name] - transitions);
            right.place_to_transitions.insert(p_name.clone(), intersecting_transitions);
            for t_name in &right.place_to_transitions[p_name] {
                let id = (p_name.clone(), t_name.clone());
                let edge = self.pt_edges.remove(&id).unwrap();
                right.pt_edges.insert(id, edge);
            }
        }
        for p_name in output_places {
            right = right.add_place(&p_name);
        }
        right
    }

    pub fn add_transition(mut self, name: &str, t: TransitionMaker) -> Self {
        self.transitions.insert(name.into(), t);
        if !self.transition_to_places.contains_key(name) {
            self.transition_to_places.insert(name.into(), HashSet::new());
        }
        self
    }
    pub fn add_place(mut self, name: &str) -> Self {
        if !self.places.contains_key(name) {
            self.places.insert(name.into(), HashMap::new());
        }
        if !self.place_to_transitions.contains_key(name) {
            self.place_to_transitions.insert(name.into(), HashSet::new());
        }
        self
    }
    pub fn set_start_tokens(mut self, place: &str, start_tokens: Vec<Token>) -> Self {
        if let Some(p) = self.places.get_mut(place) {
            for t in start_tokens.into_iter() {
                let ty = (&*t).type_id();
                if !p.contains_key(&ty) {
                    p.insert(ty.clone(), VecDeque::new());
                }
                p.get_mut(&ty).unwrap().push_back(t);
            }
        } else {
            self = self
                .add_place(place.into())
                .set_start_tokens(place, start_tokens);
        }
        self
    }
    pub fn place_to_transition(mut self, place: &str, edge: &str, transition: &str) -> Self {
        if let Some(s) = self.place_to_transitions.get_mut(place) {
            s.insert(transition.into());
        } else {
            if !self.places.contains_key(place) {
                self.places.insert(place.into(), HashMap::new());
            }
            self.place_to_transitions.insert(place.into(), HashSet::new());
            self.place_to_transitions
                .get_mut(place)
                .unwrap()
                .insert(transition.into());
        };
        self.pt_edges
            .insert((place.into(), transition.into()), edge.into());
        self
    }
    pub fn transition_to_place(mut self, transition: &str, edge: &str, place: &str) -> Self {
        if let Some(s) = self.transition_to_places.get_mut(transition) {
            s.insert(place.into());
        } else {
            if !self.places.contains_key(place) {
                self.places.insert(place.into(), HashMap::new());
            }
            self.transition_to_places.insert(transition.into(), HashSet::new());
            self.transition_to_places.get_mut(transition)
                .unwrap()
                .insert(place.into());
        }
        self.tp_edges
            .insert((transition.into(), place.into()), edge.into());
        self
    }
    pub fn pseudo_hash(&self) -> u64 {
        let mut transitions = self.transitions.keys().collect::<Vec<_>>();
        transitions.sort();
        let mut places = self.places.keys().collect::<Vec<_>>();
        places.sort();
        let mut transitions_to_places = vec![];
        for (t, ps) in self.transition_to_places.iter().sorted_by_key(|x| x.0) {
            let mut ps = ps.iter().collect::<Vec<_>>();
            ps.sort();
            transitions_to_places.push((t, ps));
        }
        let mut places_to_transitions = vec![];
        for (p, ts) in self.place_to_transitions.iter().sorted_by_key(|x| x.0) {
            let mut ts = ts.iter().collect::<Vec<_>>();
            ts.sort();
            places_to_transitions.push((p, ts));
        }
        let mut pt_edges: Vec<((String, String), String)> = vec![];
        for (pt, e) in self.pt_edges.iter().sorted_by_key(|x| x.0) {
            pt_edges.push((pt.clone(), e.clone()));
        }
        let mut tp_edges: Vec<((String, String), String)> = vec![];
        for (tp, e) in self.tp_edges.iter().sorted_by_key(|x| x.0) {
            tp_edges.push((tp.clone(), e.clone()));
        }
        let mut s = DefaultHasher::new();
        let t = (
            transitions,
            places,
            transitions_to_places,
            places_to_transitions,
            pt_edges,
            tp_edges,
        );
        t.hash(&mut s);
        s.finish()
    }
    pub fn as_dot(&self, multi_net: bool) -> (String, String) {
        let mut dot = String::new();
        for t in self.transitions.keys() {
            dot += &format!("{}[label=\"{}\" shape=rectangle];\n", t, t);
        }
        for p in self.places.keys() {
            if multi_net {
                if self.place_to_transitions[p].len() > 0 {
                    dot += &format!("{}[label=\"{}\" shape=ellipse];\n", p, p);
                }
            } else {
                dot += &format!("{}[label=\"{}\" shape=ellipse];\n", p, p);
            }
        }
        let mut dot_edges = String::new();
        for connection_set in [&self.pt_edges, &self.tp_edges] {
            for ((source, sink), name) in connection_set {
                dot_edges += &format!("{} -> {}[label=\"{}\"];\n", source, sink, name);
            }
        }
        (dot, dot_edges)
    }
    pub fn png(&self) -> PathBuf {
        let mut dot: String = "digraph  {\n".into();
        let (dot_nodes, dot_edges) = &self.as_dot(false);
        dot += dot_nodes;
        dot += dot_edges;
        dot += "}";
        graphviz(&dot, self.pseudo_hash())
    }
}
pub fn graphviz(dot: &String, hash: u64) -> PathBuf {
    let graph_cache = env::current_exe()
        .expect("Getting current exe")
        .as_path()
        .parent()
        .unwrap()
        .join(Path::new("graph_png_cache"));
    if !graph_cache.exists() {
        std::fs::create_dir(graph_cache.clone()).unwrap();
    }
    let png_file_path = graph_cache.join(format!("{}.png", hash));
    if !png_file_path.exists() {
        let mut dot_file = NamedTempFile::new().unwrap();
        dot_file.write_all(dot.as_bytes()).unwrap();
        dot_file.flush().unwrap();
        Command::new("dot")
            .arg(dot_file.path())
            .arg("-Tpng:cairo:cairo")
            .arg("-o")
            .arg(&png_file_path)
            .status()
            .expect("dot failed");
    }
    png_file_path
}

use std::fmt;
use std::fmt::Debug;

impl Debug for Net {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Net")
            .field("transitions", &self.transitions.iter().map(|(k, _)| k).collect::<Vec<_>>())
            .field("places", &self.places)
            .field("transition_to_places", &self.transition_to_places)
            .field("place_to_transitions", &self.place_to_transitions)
            .field("pt_edges", &self.pt_edges)
            .field("tp_edges", &self.tp_edges)
            .finish()
    }
}
