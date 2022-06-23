use mnet_lib::{Place, GraphMaker, GraphRunner};
use plotmux::{plotsink::PlotSink};
use mnet_macro::MnetPlace;
use std::{thread, time};

#[derive(MnetPlace)]
#[mnet_place(f, f64, f64)]
struct Sin;
impl Sin {
    fn f(&mut self, _p: &PlotSink, t: f64) -> f64 {
        _p.plot_series_2d("sin(t)".into(), t, t.sin());
        thread::sleep(time::Duration::from_millis(10));
        t + 0.01
    }
}

fn main() {
    let mut g = GraphMaker::make(); g
        .set_start_tokens::<f64>("time", vec![0.])
        .edge_to_place("time", "sin")
        .add_place("sin", Box::new(Sin{}))
        .place_to_edge("sin", "time")
    ;
    println!("{:?}", GraphRunner::from_maker(g).run());
}
