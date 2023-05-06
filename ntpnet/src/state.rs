use bimap::BiMap;
use crossbeam_channel::{Receiver, Select, Sender};
use std::any::TypeId;
use std::collections::{HashMap, HashSet, VecDeque};
use std::mem;

use crate::Token;
use plotmux::plotsink::PlotSink;

#[derive(Debug)]
pub struct StateDelta {
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
    pub fn take(
        self,
    ) -> (
        HashSet<(String, TypeId)>,
        HashMap<(String, TypeId), &'static str>,
    ) {
        (self.sub, self.add)
    }
}

pub enum StateBlockable {
    Tokens((TypeId, Token)),
    Terminate(()),
}

#[derive(Debug)]
pub struct State {
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
    pub fn make(
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
    pub fn take_places(self) -> HashMap<String, HashMap<TypeId, VecDeque<Token>>> {
        self.places
    }
    pub fn block_rx(&mut self) -> bool {
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
    pub fn try_rx(&mut self) -> bool {
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
    pub fn binary(
        &mut self,
        plot: Option<(&mut PlotSink, f64)>,
    ) -> (bool, &HashSet<(String, TypeId)>) {
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
    pub fn pop(&mut self, p_ty: &(String, TypeId)) -> Token {
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
    pub fn push(&mut self, p_ty: &(String, TypeId), t: Token) {
        self.state_delta.push(p_ty, (*t).type_name());
        if let Some(out_place) = self.output_places.get_mut(&p_ty.0) {
            out_place
                .send(StateBlockable::Tokens((p_ty.1.clone(), t)))
                .unwrap();
        } else {
            self.push_local(p_ty, t);
        }
    }
    pub fn state_delta_complete(&mut self) {
        let mut temp = StateDelta::make();
        mem::swap(&mut temp, &mut self.state_delta);
        self.state_delta_notification.send(temp).unwrap();
        self.state_delta = StateDelta::make();
    }
}
