use std::any::{Any, TypeId, type_name};
use std::collections::{HashMap, HashSet};
use std::vec::Vec;
use std::sync::Arc;

pub trait Place {
    fn in_type(&self) -> TypeId;
    fn out_types(&self) -> HashSet<TypeId>;
    fn out_types_names(&self) -> HashSet<String>;
    fn run(
        &mut self,
        x: Box<dyn Any>,
        out_map: &mut HashMap::<TypeId, Arc<Edge>>,
    );
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
    edges: HashMap<String, Arc<Edge>>,
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
    pub fn add_place(&mut self, name: String, p: Box<dyn Place>) -> &mut Self {
        self.places.insert(name.clone(), p);
        self.places_to_edges.insert(name, HashSet::new());
        self
    }
    pub fn add_edge<T: 'static>(&mut self, name: String) -> &mut Self {
        self.edges.insert(name.clone(), Arc::new(Edge {
                _name: name.clone(),
                type_name: type_name::<T>().into(),
                type_id: TypeId::of::<T>(),
                vec: vec![],
        }));
        self.edges_to_places.insert(name, HashSet::new());
        self
    }
    pub fn set_start_tokens<T: 'static>(&mut self, edge: String, mut start_tokens: Vec<T>) -> &mut Self {
        match self.edges.get_mut(&edge) {
            Some(e) => {
                for t in start_tokens.drain(..) {
                    Arc::get_mut(e).unwrap().push(Box::new(t));
                }
            }
            None => {
                self.add_edge::<T>(edge.clone()).set_start_tokens::<T>(edge, start_tokens);
            }
        }
        self
    }
    pub fn place_to_edge(&mut self, place: String, edge: String) -> &mut Self {
        match self.places_to_edges.get_mut(&place) {
            Some(s) => {
                s.insert(edge);
            },
            None => {
                self.places_to_edges.insert(place.clone(), HashSet::new());
                self.places_to_edges.get_mut(&place).unwrap().insert(edge);
            }
        };
        self
    }
    pub fn edge_to_place(&mut self, edge: String, place: String) -> &mut Self {
        match self.edges_to_places.get_mut(&edge) {
            Some(s) => {
                s.insert(place);
            },
            None => {
                self.edges_to_places.insert(edge.clone(), HashSet::new());
                self.edges_to_places.get_mut(&edge).unwrap().insert(place);
            }
        };
        self
    }
    pub fn to_runner(&mut self) -> GraphRunner {
        let mut places = HashMap::new();
        for (place_name, p) in self.places.drain() {
            let in_edges = {
                let mut in_edges = vec![];
                for (e, places) in &self.edges_to_places {
                    if places.contains(&place_name) {
                        in_edges.push(self.edges[e].clone());
                        assert_eq!(p.in_type(), self.edges[e].type_id);
                    }
                }
                in_edges
            };
            let out_edges = {
                let mut out_edges : HashMap<TypeId, Arc<Edge>> = HashMap::new();
                for edge_name in &self.places_to_edges[&place_name] {
                    let edge = self.edges[edge_name].clone();
                    assert!(!out_edges.contains_key(&edge.type_id));
                    out_edges.insert(edge.type_id, edge);
                }
                assert_eq!(out_edges.clone().into_keys().collect::<HashSet<TypeId>>(), p.out_types(),
                    "Place: {:?} has out edges: {:?}, but {:?} are not connected!", place_name,
                    p.out_types_names(),
                    p.out_types_names().difference(&out_edges.iter().map(|(_, e)| e.type_name.clone()).collect::<HashSet<String>>())
                );
                out_edges
            };
            println!("{:?}, ({:?}, ..., {:?})", place_name, in_edges, out_edges);
            places.insert(place_name, (in_edges, p, out_edges));
        }
        GraphRunner { places }
    }
}

pub struct GraphRunner {
    places: HashMap<String, (Vec<Arc<Edge>>, Box<dyn Place>, HashMap<TypeId, Arc<Edge>>)>,
}
impl GraphRunner {
    pub fn run(&mut self) {
        let mut continue_executing = true;
        while continue_executing {
            continue_executing = false;
            for (_place_name, (in_edges, place, out_edges)) in self.places.iter_mut() {
                for e in in_edges {
                    if e.len() > 0 {
                        println!("running {:?} from {:?}", _place_name, e._name);
                        place.run(Arc::get_mut(e).unwrap().pop(), out_edges);
                        continue_executing = true;
                    }
                }
            }
        }
    }
}
