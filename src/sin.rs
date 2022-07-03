use mnet_lib::{Place, graph};
use plotmux::{plotsink::PlotSink, plotmux::PlotMux};
use mnet_macro::MnetPlace;
use std::{thread, time};

#[derive(MnetPlace)]
#[mnet_place(f, f64, f64)]
struct Sin {
    p: PlotSink,
}
impl Sin {
    fn f(&mut self, t: f64) -> f64 {
        self.p.plot_series_2d("sin(t)".into(), t, t.sin());
        thread::sleep(time::Duration::from_millis(10));
        t + 0.01
    }
}

fn main() {
    let mut plotmux = PlotMux::make();
    let g = graph::Maker::make()
        .set_start_tokens::<f64>("time", vec![0.])
        .edge_to_place("time", "sin")
        .add_place("sin", Box::new(Sin{ p: plotmux.add_plot_sink("sin".into())}))
        .place_to_edge("sin", "time")
    ;
    plotmux.make_ready(&g.png());
    thread::spawn(move || plotmux.spin());
    println!("{:?}", graph::Runner::from_maker(g).run());
}
