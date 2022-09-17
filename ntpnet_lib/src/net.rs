use std::collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque};
use bimap::BiMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::hash::{Hash, Hasher};
use std::env;
use tempfile::NamedTempFile;
use itertools::Itertools;
use std::io::Write;
use std::any::TypeId;

use crate::{TransitionMaker, Token};

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
    pub fn add_transition(mut self, name: &str, t: TransitionMaker) -> Self {
        self.transitions.insert(name.into(), t);
        self.transition_to_places.insert(name.into(), HashSet::new());
        self
    }
    pub fn add_place(mut self, name: &str) -> Self {
        self.places.insert(name.into(), HashMap::new());
        self.place_to_transitions.insert(name.into(), HashSet::new());
        self
    }
    pub fn set_start_tokens(
        mut self,
        place: &str,
        mut start_tokens: Vec<Token>,
    ) -> Self {
        if let Some(p) = self.places.get_mut(&place.to_string()) {
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
        if let Some(s) = self.place_to_transitions.get_mut(&place.to_string()) {
            s.insert(transition.into());
        } else {
            self.place_to_transitions.insert(place.into(), HashSet::new());
            self.place_to_transitions
                .get_mut(&place.to_string())
                .unwrap()
                .insert(transition.into());
        };
        self.pt_edges.insert((place.into(), transition.into()), edge.into());
        self
    }
    pub fn transition_to_place(mut self, transition: &str, edge: &str, place: &str) -> Self {
        if let Some(s) = self.transition_to_places.get_mut(&transition.to_string()) {
                s.insert(place.into());
        } else {
            self.transition_to_places.insert(place.into(), HashSet::new());
            self.transition_to_places
                .get_mut(&transition.to_string())
                .unwrap()
                .insert(place.into());
        }
        self.tp_edges.insert((transition.into(), place.into()), edge.into());
        self
    }
    fn pseudo_hash(&self) -> u64 {
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
        let mut tp_edges: Vec<((String, String), String)>= vec![];
        for (tp, e) in self.tp_edges.iter().sorted_by_key(|x| x.0) {
            tp_edges.push((tp.clone(), e.clone()));
        }
        let mut s = DefaultHasher::new();
        let t = (transitions, places, transitions_to_places, places_to_transitions, pt_edges, tp_edges);
        t.hash(&mut s);
        s.finish()
    }
    pub fn png(&self) -> PathBuf {
        let graph_cache = env::current_exe()
            .expect("Getting current exe")
            .as_path()
            .parent()
            .unwrap()
            .join(Path::new("graph_png_cache"));
        if !graph_cache.exists() {
            std::fs::create_dir(graph_cache.clone()).unwrap();
        }
        let png_file_path = graph_cache.join(format!("{}.png", self.pseudo_hash()));
        if !png_file_path.exists() {
            let dot = {
                let mut dot: String = "digraph MNet {\n".into();
                for p in self.transitions.keys() {
                    dot += &format!("{}[label=\"{}\" shape=rectangle];\n", p, p);
                }
                for e in self.places.keys() {
                    dot += &format!("{}[label=\"{}\" shape=ellipse];\n", e, e);
                }
                for connection_set in [&self.pt_edges, &self.tp_edges] {
                    for ((source, sink), name) in connection_set {
                        dot += &format!("{} -> {}[label=\"{}\"];\n", source, sink, name);
                    }
                }
                dot += "}";
                dot
            };
            let mut dot_file = NamedTempFile::new().unwrap();
            dot_file.write_all(dot.as_bytes()).unwrap();
            dot_file.flush().unwrap();
            Command::new("dot")
                .arg(dot_file.path())
                .arg("-Tpng")
                .arg("-o")
                .arg(&png_file_path)
                .status()
                .expect("dot failed");
        }
        png_file_path
    }
}
