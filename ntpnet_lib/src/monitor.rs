use std::thread;
use defer::defer;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::any::TypeId;
use std::time::Instant;
use crossbeam_channel::{Receiver, Sender};

use crate::{PlotOptions, state::{StateBlockable, StateDelta}};
use plotmux::plotsink::PlotSink;

pub fn monitor_thread(
    start_state: HashMap<(String, TypeId), (i64, &'static str)>,
    nonblocking_states: HashSet<BTreeSet<(String, TypeId)>>,
    state_delta_monitor: Receiver<StateDelta>,
    exit_txs: Vec<Sender<StateBlockable>>,
    mut plot_sink: PlotSink,
    plot_options: PlotOptions,
) -> impl Drop {
    let t = thread::Builder::new()
        .name("monitor".into())
        .spawn(move || {
            for ((place, _ty), (len, ty_name)) in &start_state {
                if plot_options.monitor {
                    plot_sink.plot_series_2d(
                        "pseudo-state",
                        &format!("{}/{}", place, ty_name),
                        0.0,
                        *len as f64,
                    );
                }
            }
            let mut state = start_state;
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
                if let Ok(state_delta) = state_delta_monitor.recv() {
                    let now = (Instant::now() - start).as_secs_f64();
                    let (sub, add) = state_delta.take();
                    for s in sub {
                        *&mut state.get_mut(&s).unwrap().0 -= 1;
                        if !add.contains_key(&s) && plot_options.monitor {
                            plot_sink.plot_series_2d(
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
                    for ((place, ty), ty_name) in add {
                        let key = (place, ty);
                        if !state.contains_key(&key) {
                            state.insert(key.clone(), (1, ty_name));
                            state_binary.insert(key.clone());
                        } else {
                            *&mut state.get_mut(&key).unwrap().0 += 1;
                        }
                        if plot_options.monitor {
                            plot_sink.plot_series_2d(
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
                if let Err(_) = tx.send(StateBlockable::Terminate(())) {
                    plot_sink
                        .println(&format!("failed to terminate work-cluster-{}", i));
                }
            }
            if plot_options.monitor {
                plot_sink.println(&format!(
                    "exiting with state: {:?}",
                    state
                        .iter()
                        .filter(|((_, _), (s, _))| *s != 0)
                        .collect::<Vec<_>>()
                ));
            }
        })
    .expect("unable to spawn monitor thread");
    defer(|| t.join().expect("unable to join monitor thread"))
}
