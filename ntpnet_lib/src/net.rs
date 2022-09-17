use std::collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::hash::{Hash, Hasher};
use std::env;
use tempfile::NamedTempFile;
use itertools::Itertools;
use std::io::Write;


use crate::{TransitionMaker, Token};

pub struct Net {
    pub transitions: HashMap<String, TransitionMaker>,
    pub places: HashMap<String, VecDeque<Token>>,
    pub transitions_to_places: HashMap<String, HashSet<(String, String)>>,
    pub places_to_transitions: HashMap<String, HashSet<(String, String)>>,
}
impl Net {
    pub fn make() -> Self {
        Self {
            transitions: HashMap::new(),
            places: HashMap::new(),
            transitions_to_places: HashMap::new(),
            places_to_transitions: HashMap::new(),
        }
    }
    pub fn add_transition(mut self, name: &str, t: TransitionMaker) -> Self {
        self.transitions.insert(name.into(), t);
        self.transitions_to_places.insert(name.into(), HashSet::new());
        self
    }
    pub fn add_place(mut self, name: &str) -> Self {
        self.places.insert(name.into(), VecDeque::new());
        self.places_to_transitions.insert(name.into(), HashSet::new());
        self
    }
    pub fn set_start_tokens(
        mut self,
        place: &str,
        mut start_tokens: Vec<Token>,
    ) -> Self {
        if let Some(p) = self.places.get_mut(&place.to_string()) {
            for t in start_tokens.drain(..) {
                p.push_back(Box::new(t));
            }
        } else {
            self = self
                .add_place(place.into())
                .set_start_tokens(place, start_tokens);
        }
        self
    }
    pub fn place_to_transition(mut self, place: &str, edge: &str, transition: &str) -> Self {
        if let Some(s) = self.places_to_transitions.get_mut(&place.to_string()) {
            s.insert((edge.into(), transition.into()));
        } else {
            self.places_to_transitions.insert(place.into(), HashSet::new());
            self.places_to_transitions
                .get_mut(&place.to_string())
                .unwrap()
                .insert((edge.into(), transition.into()));
        };
        self
    }
    pub fn transition_to_place(mut self, transition: &str, edge: &str, place: &str) -> Self {
        if let Some(s) = self.transitions_to_places.get_mut(&transition.to_string()) {
                s.insert((edge.into(), place.into()));
        } else {
            self.transitions_to_places.insert(place.into(), HashSet::new());
            self.transitions_to_places
                .get_mut(&transition.to_string())
                .unwrap()
                .insert((edge.into(), place.into()));
        }
        self
    }
    fn pseudo_hash(&self) -> u64 {
        let mut transitions = self.transitions.keys().collect::<Vec<_>>();
        transitions.sort();
        let mut places = self.places.keys().collect::<Vec<_>>();
        places.sort();
        let mut transitions_to_places = vec![];
        for (t, e_ps) in self.transitions_to_places.iter().sorted_by_key(|x| x.0) {
            let mut e_ps = e_ps.iter().collect::<Vec<_>>();
            e_ps.sort();
            transitions_to_places.push((t, e_ps));
        }
        let mut places_to_transitions = vec![];
        for (p, e_ts) in self.places_to_transitions.iter().sorted_by_key(|x| x.0) {
            let mut e_ts = e_ts.iter().collect::<Vec<_>>();
            e_ts.sort();
            places_to_transitions.push((p, e_ts));
        }
        let mut s = DefaultHasher::new();
        let t = (transitions, places, transitions_to_places, places_to_transitions);
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
                for connection_set in [&self.transitions_to_places, &self.places_to_transitions] {
                    for (source, sinks) in connection_set {
                        for (edge, sink) in sinks {
                            dot += &format!("{} -> {}[label=\"{}\"];\n", source, sink, edge);
                        }
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
