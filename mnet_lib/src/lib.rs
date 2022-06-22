use std::any::{Any, TypeId, type_name};
use std::collections::{HashMap, HashSet};
use std::vec::Vec;
use sha1::{Sha1, Digest};

pub struct Printer {
    prefix: String,
    color: (u8, u8, u8),
}
impl Printer {
    fn make(prefix: String) -> Self {
        let normalize = |x: f32| (((x / 255.0) * 155.0) + 100.0) as u8;
        let mut hasher = Sha1::new();
        hasher.update(&prefix);
        let digest = hasher.finalize();
        let color = (
            normalize(digest[0].into()),
            normalize(digest[2].into()),
            normalize(digest[4].into()),
        );
        Self { prefix, color }
    }
    pub fn println(&self, s: &str) {
        println!("\x1b[38;2;{};{};{}m[{}]\x1b[0m: {}", self.color.0, self.color.1, self.color.2, self.prefix, s);
    }
}

pub trait Place {
    fn in_type(&self) -> TypeId;
    fn out_types(&self) -> HashSet<TypeId>;
    fn out_types_names(&self) -> HashSet<String>;
    fn run(&mut self, p: &Printer, x: Box<dyn Any>, out_map: &mut HashMap::<TypeId, Edge>);
}

#[derive(Debug)]
pub struct Edge {
    _name: String,
    type_name: String,
    type_id : TypeId,
    vec : Vec<Box<dyn Any>>,
}
impl Edge {
    pub fn push(&mut self, x: Box<dyn Any>) {
        assert_eq!((&*x).type_id(), self.type_id);
        self.vec.push(x);
    }
    pub fn pop(&mut self) -> Box<dyn Any> {
        self.vec.pop().unwrap()
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
}

pub struct GraphMaker {
    places: HashMap<String, Box<dyn Place>>,
    edges: HashMap<String, Edge>,
    places_to_edges: HashMap<String, HashSet<String>>,
    edges_to_places: HashMap<String, HashSet<String>>,

}
impl GraphMaker {
    pub fn make() -> Self {
        Self {
            places: HashMap::new(),
            edges: HashMap::new(),
            places_to_edges: HashMap::new(),
            edges_to_places: HashMap::new(),
        }
    }
    pub fn add_place(&mut self, name: &str, p: Box<dyn Place>) -> &mut Self {
        self.places.insert(name.into(), p);
        self.places_to_edges.insert(name.into(), HashSet::new());
        self
    }
    pub fn add_edge<T: 'static>(&mut self, name: &str) -> &mut Self {
        self.edges.insert(name.into(), Edge {
                _name: name.into(),
                type_name: type_name::<T>().into(),
                type_id: TypeId::of::<T>(),
                vec: vec![],
        });
        self.edges_to_places.insert(name.into(), HashSet::new());
        self
    }
    pub fn set_start_tokens<T: 'static>(&mut self, edge: &str, mut start_tokens: Vec<T>) -> &mut Self {
        match self.edges.get_mut(&edge.to_string()) {
            Some(e) => {
                for t in start_tokens.drain(..) {
                    e.push(Box::new(t));
                }
            }
            None => {
                self.add_edge::<T>(edge.into()).set_start_tokens::<T>(edge, start_tokens);
            }
        }
        self
    }
    pub fn place_to_edge(&mut self, place: &str, edge: &str) -> &mut Self {
        match self.places_to_edges.get_mut(&place.to_string()) {
            Some(s) => {
                s.insert(edge.into());
            },
            None => {
                self.places_to_edges.insert(place.into(), HashSet::new());
                self.places_to_edges.get_mut(&place.to_string()).unwrap().insert(edge.into());
            }
        };
        self
    }
    pub fn edge_to_place(&mut self, edge: &str, place: &str) -> &mut Self {
        match self.edges_to_places.get_mut(&edge.to_string()) {
            Some(s) => {
                s.insert(place.into());
            },
            None => {
                self.edges_to_places.insert(edge.into(), HashSet::new());
                self.edges_to_places.get_mut(&edge.to_string()).unwrap().insert(place.into());
            }
        };
        self
    }
}

pub struct GraphRunner {
    places: HashMap<String, (Printer, HashSet<String>, Box<dyn Place>, HashMap<TypeId, String>)>,
    edges: HashMap<String, Edge>,
}
impl GraphRunner {
    pub fn from_maker(mut maker: GraphMaker) -> Self {
        let mut places = HashMap::new();
        for (place_name, p) in maker.places.drain() {
            let in_edges = {
                let mut in_edges = HashSet::new();
                for (e, places) in &maker.edges_to_places {
                    if places.contains(&place_name) {
                        in_edges.insert(e.clone());
                        assert_eq!(p.in_type(), maker.edges[e].type_id);
                    }
                }
                assert!(in_edges.len() > 0);
                in_edges
            };
            let out_edges = {
                let mut out_edges : HashMap<TypeId, String> = HashMap::new();
                for edge_name in &maker.places_to_edges[&place_name] {
                    //let edge = self.edges[edge_name].clone();
                    assert!(!out_edges.contains_key(&maker.edges[edge_name].type_id));
                    out_edges.insert(maker.edges[edge_name].type_id, edge_name.clone());
                }
                assert_eq!(out_edges.clone().into_keys().collect::<HashSet<TypeId>>(), p.out_types(),
                    "Place: {:?} has out edges: {:?}, but {:?} are not connected!", place_name,
                    p.out_types_names(),
                    p.out_types_names().difference(&maker.edges.iter().map(
                        |(_, e)| e.type_name.clone()
                    ).collect::<HashSet<String>>())
                );
                out_edges
            };
            places.insert(place_name.clone(), (Printer::make(place_name), in_edges, p, out_edges));
        }
        Self { places: places, edges: maker.edges }
    }
    pub fn run(mut self: Self) -> HashMap<String, Edge> {
        let mut continue_executing = true;
        while continue_executing {
            continue_executing = false;
            for (_place_name, (printer, in_edges, place, out_edges_names)) in self.places.iter_mut() {
                for e in in_edges.iter() {
                    if self.edges[e].len() > 0 {
                        let mut out_edges = {
                            let mut out_edges = HashMap::new();
                            for (t, e_name) in out_edges_names.into_iter() {
                                out_edges.insert(*t, self.edges.remove(e_name).unwrap());
                            }
                            out_edges
                        };
                        place.run(printer, self.edges.get_mut(e).unwrap().pop(), &mut out_edges);
                        for (t, e_name) in out_edges_names.into_iter() {
                            self.edges.insert(e_name.clone(), out_edges.remove(&t).unwrap());
                        }
                        assert_eq!(out_edges.len(), 0);
                        continue_executing = true;
                    }
                }
            }
        }
        self.edges
    }
}
